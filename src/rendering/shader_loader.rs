use std::{
    path::Path,
    sync::{
        mpsc::{self, channel},
        Arc, RwLock,
    },
    time::Duration,
};

use anyhow::Context;
use id_arena::{Arena, Id};
use naga::{
    back::wgsl::WriterFlags,
    valid::{Capabilities, ValidationFlags},
};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderLanguage,
};
use notify_debouncer_mini::{
    new_debouncer_opt, notify::*, DebounceEventResult, DebouncedEventKind, Debouncer,
};
use pollster::block_on;
use wgpu::{naga, PollType, RenderPipeline};

const SHADER_FOLDER: &'static str = "assets/shaders";
const SHADER_SHADER_MODULES_FOLDER: &'static str = "assets/shaders/shared";

type PipelineFactory = Box<
    dyn Sync
        + Send
        + Fn(&wgpu::Device, &ShaderDefinition, &str) -> anyhow::Result<wgpu::RenderPipeline>,
>;

#[derive(Debug, Clone)]
pub(crate) struct ShaderDefinition {
    pub name: &'static str,
    pub path: &'static str,
}

pub struct ShaderEntry {
    pipeline_id: PipelineId,
    def: ShaderDefinition,
    factory: PipelineFactory,
}

impl ShaderEntry {
    pub fn new(pipeline_id: PipelineId, def: ShaderDefinition, factory: PipelineFactory) -> Self {
        Self {
            pipeline_id,
            def,
            factory,
        }
    }
}

pub type PipelineId = Id<PipelineCacheEntry>;

#[derive(Default)]
pub struct PipelineCacheEntry(Option<wgpu::RenderPipeline>);

impl PipelineCacheEntry {
    pub fn set_pipeline(&mut self, pipeline: wgpu::RenderPipeline) {
        self.0 = Some(pipeline);
    }
}

pub struct PipelineCacheBuilder {
    shaders: Arena<ShaderEntry>,
    pipelines: Arena<PipelineCacheEntry>,
}

impl PipelineCacheBuilder {
    pub fn new() -> Self {
        Self {
            shaders: Arena::new(),
            pipelines: Arena::new(),
        }
    }
    pub fn add_shader(
        &mut self,
        shader_def: ShaderDefinition,
        factory: PipelineFactory,
    ) -> PipelineId {
        let pipeline_id = self.pipelines.alloc(PipelineCacheEntry::default());
        let shader_entry = ShaderEntry::new(pipeline_id, shader_def, factory);
        self.shaders.alloc(shader_entry);
        pipeline_id
    }

    pub fn build(self) -> PipelineCache {
        PipelineCache {
            shaders: Arc::new(self.shaders),
            pipelines: self.pipelines,
        }
    }
}

pub struct PipelineCache {
    shaders: Arc<Arena<ShaderEntry>>,
    pipelines: Arena<PipelineCacheEntry>,
}

impl PipelineCache {
    pub fn get(&self, id: PipelineId) -> &RenderPipeline {
        self.pipelines.get(id).unwrap().0.as_ref().unwrap()
    }

    pub fn get_entry_mut(&mut self, id: PipelineId) -> &mut PipelineCacheEntry {
        self.pipelines.get_mut(id).unwrap()
    }

    pub fn iter_shaders_and_pipelines_mut(
        &mut self,
    ) -> impl Iterator<Item = (&ShaderEntry, &mut PipelineCacheEntry)> {
        // This assumes that the shaders and pipelines are in sync, which should be the case
        // because the same method inserts to both arenas.
        self.shaders
            .iter()
            .map(|(_, shader_entry)| shader_entry)
            .zip(
                self.pipelines
                    .iter_mut()
                    .map(|(_, pipeline_entry)| pipeline_entry),
            )
    }
}

// Loads and compiles shaders to pipelines in a worker thread.
pub(crate) struct ShaderLoader {
    pub cache: PipelineCache,
    device: wgpu::Device,
    receiver: mpsc::Receiver<(&'static str, PipelineId, wgpu::RenderPipeline)>,
    composer: Arc<RwLock<Composer>>,
    _debouncer: Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

impl ShaderLoader {
    pub fn new(device: wgpu::Device, cache_builder: PipelineCacheBuilder) -> Self {
        let cache = cache_builder.build();

        let (send_new_pipelines, recv_new_pipelines) = channel();

        let device_loader = device.clone();

        let composer = create_composer().expect("Failed to create composer for shader loader");
        let composer = Arc::new(RwLock::new(composer));

        let shaders = cache.shaders.clone();
        let composer_clone = composer.clone();
        let mut debouncer = new_debouncer_opt(
            notify_debouncer_mini::Config::default().with_timeout(Duration::from_millis(100)),
            move |res: DebounceEventResult| {
                match res {
                    Ok(events) => {
                        for event in events {
                            if event.kind != DebouncedEventKind::Any {
                                continue;
                            }

                            // This is stupid and slow, but that's life
                            let Some(entry) = shaders
                                .iter()
                                .find(|(_, entry)| event.path.ends_with(entry.def.path))
                                .map(|(_, entry)| entry)
                            else {
                                continue;
                            };
                            match compile_file(
                                &device_loader,
                                &entry.def,
                                &entry.factory,
                                composer_clone.clone(),
                            ) {
                                Ok(pipeline) => {
                                    send_new_pipelines
                                        .send((entry.def.name, entry.pipeline_id, pipeline))
                                        .unwrap();
                                }
                                Err(e) => println!("Failed to load shader: {:?}", e),
                            }
                        }
                    }
                    Err(e) => println!("Error debouncing shader changes: {}", e),
                }
            },
        )
        .unwrap();

        let absolute_shader_folder = Path::new(SHADER_FOLDER).canonicalize().unwrap();

        let watcher = debouncer.watcher();

        watcher
            .watch(&absolute_shader_folder, RecursiveMode::Recursive)
            .unwrap();

        let mut shader_loader = Self {
            device,
            cache,
            receiver: recv_new_pipelines,
            composer,
            _debouncer: debouncer,
        };

        shader_loader
            .create_all_pipelines()
            .expect("Failed to create all pipelines");

        shader_loader
    }

    pub(crate) fn create_all_pipelines(&mut self) -> anyhow::Result<()> {
        for (shader, pipeline_entry) in self.cache.iter_shaders_and_pipelines_mut() {
            let pipeline = compile_file(
                &self.device,
                &shader.def,
                &shader.factory,
                self.composer.clone(),
            )
            .context(format!("Failed to compile shader: {}", shader.def.name))?;
            pipeline_entry.set_pipeline(pipeline);
        }
        Ok(())
    }

    pub(crate) fn load_pending_shaders(&mut self) -> anyhow::Result<()> {
        while let Ok((name, pipeline_id, pipeline)) = self.receiver.try_recv() {
            let entry = self.cache.get_entry_mut(pipeline_id);
            println!("Shader reloaded: {}", name);
            entry.set_pipeline(pipeline);
        }

        Ok(())
    }
}

fn compile_file(
    device: &wgpu::Device,
    shader_def: &ShaderDefinition,
    factory: &PipelineFactory,
    composer: Arc<RwLock<Composer>>,
) -> anyhow::Result<wgpu::RenderPipeline> {
    let path = Path::new(SHADER_FOLDER).join(shader_def.path);
    let shader_code = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read shader file {}: {}", path.display(), e))?;

    let file_path = path.to_string_lossy().to_string();

    let mut composer = composer.write().unwrap();

    let module = composer
        .make_naga_module(NagaModuleDescriptor {
            file_path: &file_path,
            source: &shader_code,
            ..Default::default()
        })
        .context("Failed to create Naga module from shader code")?;

    // We don't need to validate, because wgpu runs the validator internally.
    let validation_flags = ValidationFlags::empty();
    let info = naga::valid::Validator::new(validation_flags, Capabilities::all())
        .validate(&module)
        .context("Failed to validate Naga module")?;

    let shader_code = naga::back::wgsl::write_string(&module, &info, WriterFlags::empty())
        .context("Failed to convert Naga module to WGSL string")?;

    device.push_error_scope(wgpu::ErrorFilter::Validation);

    let pipeline = factory(device, shader_def, &shader_code);

    device
        .poll(PollType::Wait)
        .context("Failed to poll device after shader compilation.")?;

    let error = block_on(device.pop_error_scope());

    if let Some(error) = error {
        return Err(anyhow::anyhow!(
            "Shader compilation failed for {}: {}",
            shader_def.name,
            error
        ));
    };

    pipeline
}

fn create_composer() -> anyhow::Result<Composer> {
    let shared_files = std::fs::read_dir(SHADER_SHADER_MODULES_FOLDER)
        .expect("Failed to read shared shader modules directory");
    let mut composer = Composer::default();

    for entry in shared_files {
        let entry = entry.expect("Failed to read entry in shared shader modules directory");
        let path = entry.path();

        if !path.is_file() && path.extension().map_or(false, |ext| ext != "wgsl") {
            continue;
        }

        let source =
            std::fs::read_to_string(&path).expect("Failed to read shared shader module file");

        let file_path = path.to_string_lossy().to_string();

        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: &source,
                file_path: &file_path,
                language: ShaderLanguage::Wgsl,
                ..Default::default()
            })
            .context(format!("Failed to add shared shader module: {}", file_path))?;
    }

    Ok(composer)
}
