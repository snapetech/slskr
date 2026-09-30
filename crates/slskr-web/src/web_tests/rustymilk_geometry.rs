use super::*;

#[test]
fn rustymilk_primitive_state_evaluators_match_js_runtime() {
    let mut shape = RustyMilkIndexedEntry::default();
    shape
        .base_values
        .insert("enabled".to_string(), RustyMilkValue::Number(1.0));
    shape
        .base_values
        .insert("r".to_string(), RustyMilkValue::Number(0.2));
    shape
        .base_values
        .insert("rad".to_string(), RustyMilkValue::Number(0.1));
    shape.equations.init = "q1=0.2;".to_string();
    shape.equations.frame = "rad=rad+q1+bass_att*0.1; r=min(1,r+0.3);".to_string();

    let mut frame_scope = BTreeMap::new();
    frame_scope.insert("bass_att".to_string(), RustyMilkValue::Number(2.0));
    frame_scope.insert("time".to_string(), RustyMilkValue::Number(9.0));
    let evaluated_shape = evaluate_rustymilk_shape_state(&shape, &frame_scope);
    assert_eq!(
        evaluated_shape.base_values.get("rad"),
        Some(&RustyMilkValue::Number(0.5))
    );
    assert_eq!(
        evaluated_shape.base_values.get("r"),
        Some(&RustyMilkValue::Number(0.5))
    );
    assert_eq!(
        evaluated_shape.base_values.get("q1"),
        Some(&RustyMilkValue::Number(0.2))
    );
    assert!(!evaluated_shape.base_values.contains_key("bass_att"));
    assert!(!evaluated_shape.base_values.contains_key("time"));

    let mut sprite = RustyMilkIndexedEntry::default();
    sprite
        .base_values
        .insert("enabled".to_string(), RustyMilkValue::Number(1.0));
    sprite
        .base_values
        .insert("w".to_string(), RustyMilkValue::Number(0.1));
    sprite.equations.init = "q1=0.2;".to_string();
    sprite.equations.frame = "w=w+q1+bass_att*0.1;".to_string();
    let evaluated_sprite = evaluate_rustymilk_sprite_state(&sprite, &frame_scope);
    assert_eq!(
        evaluated_sprite.base_values.get("w"),
        Some(&RustyMilkValue::Number(0.5))
    );
    assert_eq!(
        evaluated_sprite.base_values.get("q1"),
        Some(&RustyMilkValue::Number(0.2))
    );
    assert!(!evaluated_sprite.base_values.contains_key("bass_att"));
}

#[test]
fn rustymilk_custom_wave_vertices_match_js_point_mapping() {
    let mut wave = RustyMilkIndexedEntry::default();
    wave.base_values
        .insert("enabled".to_string(), RustyMilkValue::Number(1.0));
    wave.base_values
        .insert("samples".to_string(), RustyMilkValue::Number(3.0));
    wave.equations.init = "q1=0.2;".to_string();
    wave.equations.frame = "a=q1+0.3;".to_string();
    wave.equations.point = "x=i; y=0.5+sample*0.25;".to_string();
    let mut frame_scope = BTreeMap::new();
    frame_scope.insert("bass_att".to_string(), RustyMilkValue::Number(3.0));

    let evaluated_wave = evaluate_rustymilk_wave_state(&wave, &frame_scope);
    let vertices =
        create_rustymilk_custom_wave_vertices(&evaluated_wave, &[-1.0, 0.0, 1.0], &frame_scope);
    assert_eq!(
        evaluated_wave.base_values.get("a"),
        Some(&RustyMilkValue::Number(0.5))
    );
    assert_eq!(
        evaluated_wave.base_values.get("q1"),
        Some(&RustyMilkValue::Number(0.2))
    );
    assert_eq!(vertices.len(), 6);
    assert!((vertices[0] + 1.0).abs() < 0.0001);
    assert!((vertices[1] + 0.5).abs() < 0.0001);
    assert!(vertices[2].abs() < 0.0001);
    assert!(vertices[3].abs() < 0.0001);
    assert!((vertices[4] - 1.0).abs() < 0.0001);
    assert!((vertices[5] - 0.5).abs() < 0.0001);

    let mut spectrum_wave = RustyMilkIndexedEntry::default();
    spectrum_wave
        .base_values
        .insert("samples".to_string(), RustyMilkValue::Number(3.0));
    spectrum_wave.equations.point = "x=i; y=sample;".to_string();
    let spectrum_vertices = create_rustymilk_custom_wave_vertices(
        &spectrum_wave,
        &[0.0, 128.0, 255.0],
        &BTreeMap::new(),
    );
    assert!((spectrum_vertices[1] + 1.0).abs() < 0.0001);
    assert!((spectrum_vertices[3] - ((128.0 / 255.0) * 2.0 - 1.0)).abs() < 0.0001);
    assert!((spectrum_vertices[5] - 1.0).abs() < 0.0001);
}

#[test]
fn rustymilk_main_waveform_vertices_match_js_modes() {
    let samples = [-1.0, 0.0, 1.0];
    let mut scope = BTreeMap::new();

    assert_eq!(
        rounded_vec(&create_rustymilk_waveform_vertices(&samples, &scope)),
        vec![-1.0, -1.0, 0.0, 0.0, 1.0, 1.0]
    );

    scope.insert("wave_mode".to_string(), RustyMilkValue::Number(1.0));
    scope.insert("wave_scale".to_string(), RustyMilkValue::Number(0.5));
    scope.insert("wave_y".to_string(), RustyMilkValue::Number(0.25));
    assert_eq!(
        rounded_vec(&create_rustymilk_waveform_vertices(&samples, &scope)),
        vec![-1.0, -1.0, 0.0, -0.5, 1.0, 0.0]
    );

    scope.insert("wave_mode".to_string(), RustyMilkValue::Number(2.0));
    scope.insert("wave_x".to_string(), RustyMilkValue::Number(0.75));
    assert_eq!(
        rounded_vec(&create_rustymilk_waveform_vertices(&samples, &scope)),
        vec![0.0, -1.0, 0.5, 0.0, 1.0, 1.0]
    );

    scope.insert("wave_mode".to_string(), RustyMilkValue::Number(3.0));
    scope.insert("wave_scale".to_string(), RustyMilkValue::Number(1.0));
    scope.insert("wave_x".to_string(), RustyMilkValue::Number(0.5));
    scope.insert("wave_y".to_string(), RustyMilkValue::Number(0.5));
    assert_eq!(
        rounded_vec(&create_rustymilk_waveform_vertices(&samples, &scope)),
        vec![0.17, 0.0, -0.35, 0.0, 0.53, -0.0]
    );

    scope.insert("wave_mode".to_string(), RustyMilkValue::Number(1.0));
    scope.insert("wave_scale".to_string(), RustyMilkValue::Number(1.0));
    scope.insert("wave_y".to_string(), RustyMilkValue::Number(0.5));
    scope.insert("wave_smoothing".to_string(), RustyMilkValue::Number(0.5));
    assert_eq!(
        rounded_vec(&create_rustymilk_waveform_vertices(&samples, &scope)),
        vec![-1.0, -1.0, 0.0, -0.5, 1.0, 0.5]
    );
}

#[test]
fn rustymilk_shape_vertices_match_js_geometry() {
    let mut shape = RustyMilkIndexedEntry::default();
    shape
        .base_values
        .insert("enabled".to_string(), RustyMilkValue::Number(1.0));
    shape
        .base_values
        .insert("sides".to_string(), RustyMilkValue::Number(4.0));
    shape
        .base_values
        .insert("rad".to_string(), RustyMilkValue::Number(0.25));
    shape
        .base_values
        .insert("x".to_string(), RustyMilkValue::Number(0.5));
    shape
        .base_values
        .insert("y".to_string(), RustyMilkValue::Number(0.5));
    let outline = create_rustymilk_shape_vertices(&shape);
    let fill = create_rustymilk_shape_fill_vertices(&shape);
    assert_eq!(outline.len(), 10);
    assert_eq!(fill.len(), 12);
    assert!((outline[0] - 0.25).abs() < 0.0001);
    assert!(outline[1].abs() < 0.0001);
    assert!(fill[0].abs() < 0.0001);
    assert!(fill[1].abs() < 0.0001);
}

#[test]
fn rustymilk_textured_primitives_match_js_geometry() {
    let frame = rustymilk_frame_from_source(
        r#"
            name=Textured
            wave_r=0.5
            wave_g=0.6
            wave_b=0.7
            shape00_enabled=1
            shape00_textured=1
            shape00_texture='textures\cover.png'
            shape00_sides=4
            shape00_rad=0.25
            shape00_tex_zoom=1
            sprite00_enabled=1
            sprite00_file=Sprites/flare.webp
            sprite00_x=0.5
            sprite00_y=0.5
            sprite00_w=0.25
            sprite00_h=0.125
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );
    assert_eq!(frame.textured_primitives.len(), 2);
    assert_eq!(
        frame.textured_primitives[0].texture_name,
        "'textures\\cover.png'"
    );
    assert_eq!(
        frame.textured_primitives[1].texture_name,
        "Sprites/flare.webp"
    );
    assert_eq!(
        get_rustymilk_texture_name_aliases(&frame.textured_primitives[0].texture_name),
        vec![
            "textures/cover.png".to_string(),
            "cover.png".to_string(),
            "cover".to_string()
        ]
    );
    assert_eq!(frame.textured_primitives[0].vertices.len(), 12);
    assert_eq!(frame.textured_primitives[0].uvs.len(), 12);
    assert_eq!(frame.textured_primitives[1].vertices.len(), 10);
    assert_eq!(
        frame.textured_primitives[1].uvs,
        vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn rustymilk_shape_fill_vertex_colors_match_js_edges() {
    let frame = rustymilk_frame_from_source(
        r#"
            name=Shape edge colors
            wave_a=0
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
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );
    let shape = frame
        .primitives
        .iter()
        .find(|primitive| primitive.mode == RustyMilkPrimitiveMode::TriangleFan)
        .expect("shape fill primitive");

    assert_eq!(
        rounded_vec(&shape.vertex_colors[..8]),
        vec![0.1, 0.2, 0.3, 0.4, 0.4, 0.5, 0.6, 0.2]
    );

    let batches = create_rustymilk_webgpu_frame_batches(&frame);
    assert_eq!(
        rounded_vec(&batches.filled_vertices[2..6]),
        vec![0.1, 0.2, 0.3, 0.4]
    );
    assert_eq!(
        rounded_vec(&batches.filled_vertices[8..12]),
        vec![0.4, 0.5, 0.6, 0.2]
    );
}

#[test]
fn rustymilk_borders_and_motion_vectors_match_js_geometry() {
    let frame = rustymilk_frame_from_source(
        r#"
            name=Overlay primitives
            wave_r=0.5
            wave_g=0.6
            wave_b=0.7
            ob_size=0.05
            ob_a=0.8
            ib_size=0.025
            ib_a=0.5
            mv_x=2
            mv_y=2
            mv_dx=0.1
            mv_dy=0.2
            mv_l=0.5
            mv_a=0.75
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );
    let borders = frame
        .primitives
        .iter()
        .filter(|primitive| primitive.mode == RustyMilkPrimitiveMode::Triangles)
        .collect::<Vec<_>>();
    assert_eq!(borders.len(), 2);
    assert_eq!(borders[0].vertices.len(), 48);
    assert_eq!(borders[1].vertices.len(), 48);
    let motion = frame
        .primitives
        .iter()
        .find(|primitive| primitive.mode == RustyMilkPrimitiveMode::Lines)
        .expect("motion vectors");
    assert_eq!(motion.vertices.len(), 16);
    assert_eq!(&motion.vertices[0..4], &[-1.0, -1.0, -0.9, -0.8]);
    assert!((motion.color[3] - 0.75).abs() < 0.0001);
}
