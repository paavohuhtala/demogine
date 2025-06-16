use std::{path::Path, sync::mpsc::channel, time::Duration};

use anyhow::Context;
use notify_debouncer_mini::{
    new_debouncer_opt, notify::*, DebounceEventResult, DebouncedEventKind, Debouncer,
};
use pollster::block_on;
use wgpu::PollType;

const SHADER_FOLDER: &'static str = "src/shaders";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShaderId {
    Default,
}

#[derive(Debug, Clone)]
pub(crate) struct ShaderDefinition {
    pub id: ShaderId,
    pub name: &'static str,
    path: &'static str,
}

// Loads and compiles shaders to pipelines in a worker thread.
pub(crate) struct ShaderLoader {
    pub default_shader: wgpu::RenderPipeline,
    receiver: std::sync::mpsc::Receiver<(ShaderDefinition, wgpu::RenderPipeline)>,
    _debouncer: Debouncer<ReadDirectoryChangesWatcher>,
}

impl ShaderLoader {
    pub fn new<F>(device: wgpu::Device, mut load_pipeline: F) -> Self
    where
        F: 'static
            + Clone
            + Send
            + Sync
            + FnMut(&wgpu::Device, &ShaderDefinition, &str) -> anyhow::Result<wgpu::RenderPipeline>,
    {
        const DEFAULT_SHADER_FILE: &'static str = "shader.wgsl";

        const DEFAULT_SHADER: ShaderDefinition = ShaderDefinition {
            id: ShaderId::Default,
            name: "Default Shader",
            path: DEFAULT_SHADER_FILE,
        };

        let (send_changed_shaders, recv_changed_shaders) = channel();

        let mut load_pipeline_watcher = load_pipeline.clone();
        let device_loader = device.clone();

        let mut debouncer = new_debouncer_opt(
            notify_debouncer_mini::Config::default().with_timeout(Duration::from_millis(100)),
            move |res: DebounceEventResult| {
                match res {
                    Ok(events) => {
                        for event in events {
                            if event.path.ends_with(DEFAULT_SHADER_FILE)
                                && event.kind == DebouncedEventKind::Any
                            {
                                // Load the default shader
                                println!("Reloading shader: {:?}", DEFAULT_SHADER.id);
                                match compile_file(
                                    &device_loader,
                                    &DEFAULT_SHADER,
                                    &mut load_pipeline_watcher,
                                ) {
                                    Ok(pipeline) => {
                                        send_changed_shaders
                                            .send((DEFAULT_SHADER, pipeline))
                                            .unwrap();
                                    }
                                    Err(e) => println!("Failed to load shader: {}", e),
                                }
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

        let default_shader = compile_file(&device, &DEFAULT_SHADER, &mut load_pipeline)
            .unwrap_or_else(|e| {
                println!("Failed to compile default shader: {}", e);
                panic!("Shader compilation failed");
            });

        Self {
            default_shader,
            receiver: recv_changed_shaders,
            _debouncer: debouncer,
        }
    }

    pub(crate) fn load_pending_shaders(&mut self) -> anyhow::Result<()> {
        while let Ok((shader_def, pipeline)) = self.receiver.try_recv() {
            match shader_def.id {
                ShaderId::Default => {
                    self.default_shader = pipeline;
                }
            }
        }

        Ok(())
    }
}

fn compile_file<F>(
    device: &wgpu::Device,
    shader_def: &ShaderDefinition,
    f: &mut F,
) -> anyhow::Result<wgpu::RenderPipeline>
where
    F: FnMut(&wgpu::Device, &ShaderDefinition, &str) -> anyhow::Result<wgpu::RenderPipeline>,
{
    device.push_error_scope(wgpu::ErrorFilter::Validation);

    let path = Path::new(SHADER_FOLDER).join(shader_def.path);
    let shader_code = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read shader file {}: {}", path.display(), e))?;
    let pipeline = f(device, shader_def, &shader_code);

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
