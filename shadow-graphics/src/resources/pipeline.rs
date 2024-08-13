use wgpu::{FragmentState, VertexState};

fn test(device: &wgpu::Device) {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: todo!(),
            visibility: todo!(),
            ty: wgpu::BindingType::Texture {
                sample_type: todo!(),
                view_dimension: todo!(),
                multisampled: todo!(),
            },
            count: todo!(),
        }],
    });

    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: todo!(),
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: todo!(),
        layout: todo!(),
        vertex: VertexState {
            module: todo!(),
            entry_point: todo!(),
            buffers: todo!(),
            compilation_options: Default::default(),
        },
        primitive: todo!(),
        depth_stencil: todo!(),
        multisample: todo!(),
        fragment: Some(FragmentState {
            module: todo!(),
            entry_point: todo!(),
            targets: &[Some(wgpu::ColorTargetState {
                format: todo!(),
                blend: todo!(),
                write_mask: todo!(),
            })],
            compilation_options: Default::default(),
        }),
        multiview: None,
        cache: None,
    });
}
