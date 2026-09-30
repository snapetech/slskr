use super::*;

#[test]
fn rustymilk_webgpu_vertex_packers_match_js_shapes() {
    assert_eq!(
        rounded_vec(&create_rustymilk_webgpu_triangle_list_vertices(
            &[-1.0, -1.0, 1.0, -1.0, 0.0, 1.0],
            [0.2, 0.4, 0.6, 0.8],
        )),
        vec![
            -1.0, -1.0, 0.2, 0.4, 0.6, 0.8, 1.0, -1.0, 0.2, 0.4, 0.6, 0.8, 0.0, 1.0, 0.2, 0.4, 0.6,
            0.8,
        ]
    );
    assert_eq!(
        rounded_vec(&create_rustymilk_webgpu_triangle_fan_vertices(
            &[0.0, 0.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0],
            &[1.0, 0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.6, 0.0, 0.0, 1.0, 0.7, 1.0, 1.0, 1.0, 0.8,],
            [1.0, 1.0, 1.0, 1.0],
        )),
        vec![
            0.0, 0.0, 1.0, 0.0, 0.0, 0.5, -1.0, -1.0, 0.0, 1.0, 0.0, 0.6, 1.0, -1.0, 0.0, 0.0, 1.0,
            0.7, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, -1.0, 0.0, 0.0, 1.0, 0.7, 1.0, 1.0, 1.0, 1.0,
            1.0, 0.8,
        ]
    );
    assert_eq!(
        rounded_vec(&create_rustymilk_webgpu_line_segment_vertices(
            &[-1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
            [0.1, 0.2, 0.3, 0.4],
        )),
        vec![
            -1.0, 0.0, 0.1, 0.2, 0.3, 0.4, 0.0, 0.5, 0.1, 0.2, 0.3, 0.4, 0.0, 0.5, 0.1, 0.2, 0.3,
            0.4, 1.0, 0.0, 0.1, 0.2, 0.3, 0.4,
        ]
    );
}

#[test]
fn rustymilk_webgpu_textured_vertex_packers_match_js_shapes() {
    let vertices = create_rustymilk_webgpu_textured_triangle_fan_vertices(
        &[0.0, 0.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0],
        &[0.5, 0.5, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0],
        &[
            1.0, 0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.6, 0.0, 0.0, 1.0, 0.7, 1.0, 1.0, 1.0, 0.8,
        ],
        [1.0, 1.0, 1.0, 1.0],
    );
    assert_eq!(vertices.len(), 48);
    assert_eq!(
        rounded_vec(&vertices[..24]),
        vec![
            0.0, 0.0, 0.5, 0.5, 1.0, 0.0, 0.0, 0.5, -1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.6, 1.0,
            -1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.7,
        ]
    );
}

#[test]
fn rustymilk_webgpu_domain_packers_match_js_renderer() {
    let parsed = parse_rustymilk_preset_set(
        r#"
            name=WebGPU pack
            ob_size=0.1
            ob_r=1
            ob_g=0.2
            ob_b=0.3
            ob_a=0.4
            ib_size=0.05
            ib_r=0.2
            ib_g=0.8
            ib_b=1
            ib_a=0.5
            mv_x=2
            mv_y=1
            mv_r=0.1
            mv_g=0.2
            mv_b=0.3
            mv_a=0.75
            mv_dx=0.5
            mv_dy=-0.25
            mv_l=0.1
            shape00_enabled=1
            shape00_sides=3
            shape00_rad=0.25
            shape00_r=0.1
            shape00_g=0.2
            shape00_b=0.3
            shape00_a=0.4
            shape00_r2=0.4
            shape00_g2=0.5
            shape00_b2=0.6
            shape00_a2=0.2
            sprite00_enabled=1
            sprite00_x=0.5
            sprite00_y=0.5
            sprite00_w=0.2
            sprite00_h=0.1
            sprite00_r=0.1
            sprite00_g=0.2
            sprite00_b=0.3
            sprite00_a=0.4
            "#,
        false,
    );
    let preset = &parsed.presets[0];
    let borders =
        create_rustymilk_webgpu_screen_border_vertices(&preset.base_values, [0.7, 0.7, 0.7]);
    assert_eq!(borders.len(), 288);
    assert_eq!(rounded_vec(&borders[2..6]), vec![1.0, 0.2, 0.3, 0.4]);
    let motion =
        create_rustymilk_webgpu_motion_vector_vertices(&preset.base_values, [0.7, 0.7, 0.7]);
    assert_eq!(motion.len(), 24);
    assert_eq!(rounded_vec(&motion[2..6]), vec![0.1, 0.2, 0.3, 0.75]);
    let fills = create_rustymilk_webgpu_shape_fill_vertices(&preset.shapes, [0.7, 0.7, 0.7]);
    assert_eq!(fills.len(), 54);
    assert_eq!(rounded_vec(&fills[2..6]), vec![0.1, 0.2, 0.3, 0.4]);
    assert_eq!(rounded_vec(&fills[8..12]), vec![0.4, 0.5, 0.6, 0.2]);
    let sprites = create_rustymilk_webgpu_sprite_vertices(&preset.sprites, [1.0, 1.0, 1.0]);
    assert_eq!(sprites.len(), 36);
    assert_eq!(
        rounded_vec(&sprites[..6]),
        vec![-0.2, -0.1, 0.1, 0.2, 0.3, 0.4]
    );
    let textured_sprite =
        create_rustymilk_webgpu_textured_sprite_vertices(&preset.sprites[0], [1.0, 1.0, 1.0]);
    assert_eq!(textured_sprite.len(), 48);
    assert_eq!(
        rounded_vec(&textured_sprite[..8]),
        vec![-0.2, -0.1, 0.0, 1.0, 0.1, 0.2, 0.3, 0.4]
    );
}

#[test]
fn rustymilk_webgpu_frame_batches_pack_runtime_primitives() {
    let frame = rustymilk_frame_from_source(
        r#"
            name=WebGPU frame
            ob_size=0.1
            ob_r=1
            ob_g=0.2
            ob_b=0.3
            ob_a=0.4
            mv_x=2
            mv_y=1
            mv_r=0.1
            mv_g=0.2
            mv_b=0.3
            mv_a=0.75
            mv_l=0.1
            shape00_enabled=1
            shape00_sides=3
            shape00_rad=0.25
            shape00_r=0.1
            shape00_g=0.2
            shape00_b=0.3
            shape00_a=0.4
            shape01_enabled=1
            shape01_sides=3
            shape01_rad=0.25
            shape01_r=0.2
            shape01_g=0.3
            shape01_b=0.4
            shape01_a=0.5
            shape01_textured=1
            shape01_texture=panel.png
            sprite00_enabled=1
            sprite00_x=0.5
            sprite00_y=0.5
            sprite00_w=0.2
            sprite00_h=0.1
            sprite00_r=0.1
            sprite00_g=0.2
            sprite00_b=0.3
            sprite00_a=0.4
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );
    assert!(frame
        .textured_primitives
        .iter()
        .any(|primitive| primitive.mode == RustyMilkTexturedPrimitiveMode::TriangleFan));
    assert!(frame
        .textured_primitives
        .iter()
        .any(|primitive| primitive.mode == RustyMilkTexturedPrimitiveMode::Quad));

    let batches = create_rustymilk_webgpu_frame_batches(&frame);

    assert!(batches.filled_vertices.len() >= 48);
    assert!(batches.line_vertices.len() >= 24);
    assert_eq!(batches.textured_batches.len(), 2);
    assert_eq!(batches.textured_batches[0].first_vertex, 0);
    assert_eq!(batches.textured_batches[0].texture_name, "panel.png");
    assert_eq!(batches.textured_batches[0].vertex_count, 9);
    assert_eq!(batches.textured_batches[1].first_vertex, 9);
    assert_eq!(batches.textured_batches[1].vertex_count, 6);
    assert_eq!(batches.textured_vertices.len(), 120);
    assert_eq!(
        rounded_vec(
            &batches.textured_vertices[batches.textured_batches[1].first_vertex * 8..][..8]
        ),
        vec![-0.2, -0.1, 0.0, 1.0, 0.1, 0.2, 0.3, 0.4]
    );
}

#[test]
fn rustymilk_webgpu_frame_set_batches_keep_composite_offsets() {
    let frame_set = rustymilk_frame_set_from_source(
        r#"
            [preset00]
            name=Primary
            shape00_enabled=1
            shape00_sides=3
            shape00_rad=0.2
            shape00_r=0.1
            shape00_g=0.2
            shape00_b=0.3
            [preset01]
            name=Secondary
            blend_alpha=0.4
            composite_mode=additive
            wavecode_0_enabled=1
            wavecode_0_samples=3
            shape00_enabled=1
            shape00_sides=3
            shape00_rad=0.2
            shape00_textured=1
            sprite00_enabled=1
            sprite00_w=0.2
            sprite00_h=0.1
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );
    let batches = create_rustymilk_webgpu_frame_set_batches(&frame_set);

    assert_eq!(batches.composite_batches.len(), 2);
    assert_eq!(batches.composite_batches[0].index, 0);
    assert_eq!(batches.composite_batches[0].blend_alpha, 1.0);
    assert_eq!(batches.composite_batches[0].composite_mode, "alpha");
    assert!(batches.composite_batches[0].filled_vertex_count > 0);
    assert_eq!(batches.composite_batches[1].index, 1);
    assert_eq!(batches.composite_batches[1].blend_alpha, 0.4);
    assert_eq!(batches.composite_batches[1].composite_mode, "additive");
    assert!(batches.composite_batches[1].line_vertex_count > 0);
    assert_eq!(batches.composite_batches[1].textured_batch_count, 2);
    assert_eq!(
        batches.composite_batches[1].filled_first_vertex,
        batches.composite_batches[0].filled_first_vertex
            + batches.composite_batches[0].filled_vertex_count
    );
    assert_eq!(
        batches.composite_batches[1].textured_first_vertex,
        batches.composite_batches[0].textured_first_vertex
            + batches.composite_batches[0].textured_vertex_count
    );
    assert_eq!(
        batches.composite_batches[1].textured_batch_first,
        batches.composite_batches[0].textured_batch_first
            + batches.composite_batches[0].textured_batch_count
    );
    assert_eq!(
        batches.textured_batches[batches.composite_batches[1].textured_batch_first].first_vertex,
        batches.composite_batches[1].textured_first_vertex
    );
}

#[test]
fn rustymilk_webgpu_batch_summary_json_exposes_wasm_contract() {
    let summary = rustymilk_webgpu_batch_summary_json(
        r#"
            name=WebGPU summary
            per_frame_1=q1=get_waveform(0.5);
            ob_size=0.1
            ob_r=1
            ob_g=0.2
            ob_b=0.3
            ob_a=0.4
            shape00_enabled=1
            shape00_sides=3
            shape00_rad=0.25
            shape00_r=0.1
            shape00_g=0.2
            shape00_b=0.3
            shape00_a=0.4
            shape01_enabled=1
            shape01_sides=3
            shape01_rad=0.25
            shape01_textured=1
            sprite00_enabled=1
            sprite00_x=0.5
            sprite00_y=0.5
            sprite00_w=0.2
            sprite00_h=0.1
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
        &parse_rustymilk_sample_csv("-1, 0, 1, 0.5"),
        &parse_rustymilk_sample_csv("0, .5, 1"),
    );
    let value: serde_json::Value = serde_json::from_str(&summary).unwrap();

    assert_eq!(value["backend"], "webgpu");
    assert_eq!(value["frameSet"]["presetCount"], 1);
    assert_eq!(value["frameSet"]["entries"][0]["title"], "WebGPU summary");
    assert_eq!(value["packedFrameSet"]["compositeBatches"][0]["index"], 0);
    assert_eq!(
        value["packedFrameSet"]["filledVertices"],
        value["packed"]["filledVertices"]
    );
    assert_eq!(value["frame"]["q1"], 1.0);
    assert_eq!(value["frame"]["texturedPrimitives"], 2);
    assert!(value["packed"]["filledVertices"].as_u64().unwrap() >= 15);
    assert_eq!(value["packed"]["texturedVertices"], 15);
    assert_eq!(
        value["packed"]["texturedBatches"][0]["firstVertex"],
        serde_json::Value::from(0)
    );
    assert_eq!(
        value["packed"]["texturedBatches"][1]["firstVertex"],
        serde_json::Value::from(9)
    );
    assert!(value["packed"]["texturedSample"]
        .as_array()
        .is_some_and(|items| items.len() == 24));
}
