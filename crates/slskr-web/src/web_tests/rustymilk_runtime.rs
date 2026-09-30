use super::*;

#[test]
fn rustymilk_parser_and_frame_state_are_deterministic() {
    let preset = parse_rustymilk_preset(
            "name=Test\ndecay=0.88\nwave_r=0.2\nwave_g=0.4\nwave_b=0.6\nwave_a=0.9\nwave_scale=1.5\nzoom=1.1\nrot=0.03",
        );

    assert_eq!(preset.decay, 0.88);
    assert_eq!(preset.wave_r, 0.2);
    assert_eq!(preset.wave_g, 0.4);
    assert_eq!(preset.wave_b, 0.6);
    assert_eq!(preset.wave_a, 0.9);
    assert_eq!(preset.wave_scale, 1.5);
    assert_eq!(preset.zoom, 1.1);
    assert_eq!(preset.rot, 0.03);
    assert_eq!(
        rustymilk_preset_name("name=Archive Tunnel\nwave_r=1"),
        "Archive Tunnel"
    );

    let frame = rustymilk_frame(&preset, 2.0, 0.5, 0.25, 0.75);
    assert!(frame.background_alpha > 0.0);
    assert!(frame.rotation.abs() > 0.0);
    assert!(frame.wave_radius > 0.1);
    assert!((0.5..=1.8).contains(&frame.zoom));
    assert!(frame.wave_color.0 >= 95);
    assert!(frame.wave_color.1 >= 120);
    assert!(frame.wave_color.2 >= 200);
}

#[test]
fn rustymilk_runtime_evaluates_parsed_frame_equations() {
    let frame = rustymilk_frame_from_source(
        r#"
            name=Runtime
            decay=0.88
            wave_r=0.1
            wave_g=0.2
            wave_b=0.3
            wave_scale=1.2
            zoom=1
            per_frame_1=wave_r = min(1, wave_r + bass_att * 0.2);
            per_frame_2=wave_g = 0.4 + mid_att * 0.2;
            per_frame_3=wave_b = 0.5 + treb_att * 0.2;
            per_frame_4=zoom = 1.2;
            per_frame_5=rot = 0.25;
            per_frame_6=dx = 0.05;
            shape00_enabled=1
            shape01_enabled=1
            wavecode_0_enabled=1
            "#,
        2.0,
        2.0,
        0.5,
        0.25,
    );
    assert_eq!(frame.wave_color.0, 127);
    assert_eq!(frame.wave_color.1, 127);
    assert_eq!(frame.wave_color.2, 140);
    assert_eq!(frame.shape_count, 2);
    assert_eq!(frame.waveform_count, 1);
    assert!(frame.primitives.iter().any(|primitive| {
        primitive.mode == RustyMilkPrimitiveMode::TriangleFan && primitive.vertices.len() >= 8
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        primitive.mode == RustyMilkPrimitiveMode::LineStrip && primitive.vertices.len() >= 4
    }));
    assert!((frame.zoom - 1.2).abs() < 0.0001);
    assert!((frame.rotation - 0.245).abs() < 0.0001);
    assert!((frame.dx - 0.05).abs() < 0.0001);
}

#[test]
fn rustymilk_runtime_uses_audio_samples_for_custom_waves() {
    let frame = rustymilk_frame_from_source_with_audio(
        r#"
            name=Audio reactive
            wave_r=0.2
            wave_g=0.3
            wave_b=0.4
            per_frame_1=wave_r=get_fft(0.5);
            per_frame_2=q1=bass_att+0.25;
            comp_shader=float energy = get_fft(0.5);
            comp_shader_1=ret = tex2D(sampler_main, uv).rgb * vec3(q1, energy, get_waveform(0.5));
            wavecode_0_enabled=1
            wavecode_0_samples=3
            wavecode_0_per_point1=x=i;
            wavecode_0_per_point2=y=0.5+sample*0.5;
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
        &[-1.0, 0.0, 1.0],
        &[0.0, 0.5, 1.0],
    );
    assert_eq!(frame.wave_color.0, 127);
    assert!((frame.q_registers[0] - 0.35).abs() < 0.0001);
    assert!((frame.fft_bins[32] - 0.5).abs() < 0.0001);
    assert!(frame.waveform_bins[0] < -0.9);
    assert!(frame.waveform_bins[63] > 0.9);
    assert!(frame
        .shader_source
        .contains("uniform sampler2D previousFrame;"));
    assert!(frame.shader_source.contains("float energy"));
    assert!(frame
        .shader_source
        .contains("texture(previousFrame, uv).rgb"));
    let wave = frame
        .primitives
        .iter()
        .find(|primitive| {
            primitive.mode == RustyMilkPrimitiveMode::LineStrip
                && primitive.vertices.len() == 6
                && (primitive.vertices[1] + 1.0).abs() < 0.0001
        })
        .expect("custom wave primitive");
    assert_eq!(wave.vertices.len(), 6);
    assert!((wave.vertices[1] + 1.0).abs() < 0.0001);
    assert!(wave.vertices[3].abs() < 0.0001);
    assert!((wave.vertices[5] - 1.0).abs() < 0.0001);
}

#[test]
fn rustymilk_runtime_uses_waveform_function_in_equations() {
    let frame = rustymilk_frame_from_source_with_audio(
        r#"
            name=Waveform expression
            wave_r=0.1
            wave_g=0.2
            wave_b=0.3
            per_frame_1=q1=get_waveform(0.5);
            per_frame_2=wave_r=q1;
            shape00_enabled=1
            shape00_sides=4
            shape00_per_frame1=rad=0.05+get_waveform(0.75)*0.1;
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
        &[-1.0, 0.0, 1.0, 0.5],
        &[0.0, 0.5, 1.0],
    );

    assert!((frame.q_registers[0] - 1.0).abs() < 0.0001);
    assert_eq!(frame.wave_color.0, 255);
    assert!(frame.primitives.iter().any(|primitive| {
        primitive.mode == RustyMilkPrimitiveMode::TriangleFan
            && primitive.vertices.iter().any(|value| value.abs() > 0.09)
    }));
}

#[test]
fn rustymilk_runtime_persists_q_registers_across_frames() {
    let mut runtime = RustyMilkRuntime::default();
    let source = r#"
            name=Persistent
            wave_r=0
            wave_g=0.2
            wave_b=0.3
            init_1=q1=1;
            per_frame_1=q1=q1+1;
            per_frame_2=wave_r=q1/10;
            "#;

    let first = runtime.render_source(source, 1.0, 0.1, 0.2, 0.3);
    let second = runtime.render_source(source, 2.0, 0.1, 0.2, 0.3);

    assert!((first.q_registers[0] - 2.0).abs() < 0.0001);
    assert!((second.q_registers[0] - 3.0).abs() < 0.0001);
    assert!(second.wave_color.0 > first.wave_color.0);
}

#[test]
fn rustymilk_runtime_feeds_mouse_state_into_equations() {
    let input = RustyMilkInputState {
        mouse_down: 1.0,
        mouse_dx: 0.2,
        mouse_dy: -0.1,
        mouse_x: 0.75,
        mouse_y: 0.25,
    };
    let frame = rustymilk_frame_from_source_with_audio_and_input(
        r#"
            name=Mouse input
            wave_r=0
            wave_g=0
            wave_b=0
            per_frame_1=wave_r=mouse_x;
            per_frame_2=wave_g=mouse_y;
            per_frame_3=q1=mouse_down+mouse_dx+mouse_dy;
            shape00_enabled=1
            shape00_sides=3
            shape00_rad=0.1
            shape00_per_frame1=x=mouse_x;
            shape00_per_frame2=y=mouse_y;
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
        &[],
        &[],
        input,
    );

    assert_eq!(frame.wave_color.0, (0.75 * 255.0) as u8);
    assert_eq!(frame.wave_color.1, (0.25 * 255.0) as u8);
    assert!((frame.q_registers[0] - 1.1).abs() < 0.0001);
    let shape = frame
        .primitives
        .iter()
        .find(|primitive| primitive.mode == RustyMilkPrimitiveMode::TriangleFan)
        .expect("shape");
    assert!(shape.vertices[0] > 0.4);
    assert!(shape.vertices[1] < -0.4);
}

#[test]
fn rustymilk_runtime_resets_when_source_changes() {
    let mut runtime = RustyMilkRuntime::default();
    let first_source = r#"
            name=First
            init_1=q1=4;
            per_frame_1=q1=q1+2;
            per_frame_2=wave_r=q1/10;
            "#;
    let second_source = r#"
            name=Second
            init_1=q1=1;
            per_frame_1=q1=q1+1;
            per_frame_2=wave_r=q1/10;
            "#;

    let first = runtime.render_source(first_source, 1.0, 0.1, 0.2, 0.3);
    let accumulated = runtime.render_source(first_source, 2.0, 0.1, 0.2, 0.3);
    let reset = runtime.render_source(second_source, 3.0, 0.1, 0.2, 0.3);

    assert!((first.q_registers[0] - 6.0).abs() < 0.0001);
    assert!((accumulated.q_registers[0] - 8.0).abs() < 0.0001);
    assert!((reset.q_registers[0] - 2.0).abs() < 0.0001);
    assert!(reset.wave_color.0 < accumulated.wave_color.0);
}

#[test]
fn rustymilk_runtime_persists_indexed_entry_state() {
    let mut runtime = RustyMilkRuntime::default();
    let source = r#"
            name=Entry state
            shape00_enabled=1
            shape00_sides=4
            shape00_init1=q2=1;
            shape00_per_frame1=q2=q2+1;
            shape00_per_frame2=rad=0.05+q2*0.01;
            "#;

    let first = runtime.render_source(source, 1.0, 0.1, 0.2, 0.3);
    let second = runtime.render_source(source, 2.0, 0.1, 0.2, 0.3);

    assert!((first.q_registers[1] - 2.0).abs() < 0.0001);
    assert!((second.q_registers[1] - 3.0).abs() < 0.0001);
    assert!(second.primitives.iter().any(|primitive| {
        primitive.mode == RustyMilkPrimitiveMode::TriangleFan
            && primitive.vertices.iter().any(|value| value.abs() > 0.07)
    }));
}

#[test]
fn rustymilk_runtime_merges_entry_q_registers_in_js_order() {
    let mut runtime = RustyMilkRuntime::default();
    let source = r#"
            name=Entry q order
            wavecode_0_enabled=1
            wavecode_0_samples=3
            wavecode_0_init1=q5=0.2;
            wavecode_0_per_frame1=q5=q5+0.1;
            wavecode_0_per_point1=x=i;
            wavecode_0_per_point2=y=sample;
            shape00_enabled=1
            shape00_sides=4
            shape00_rad=0.01
            shape00_per_frame1=rad=q5;
            "#;

    let frame = runtime.render_source_with_audio(
        source,
        1.0,
        0.1,
        0.2,
        0.3,
        &[-1.0, 0.0, 1.0],
        &[0.0, 0.5, 1.0],
    );

    assert!((frame.q_registers[4] - 0.3).abs() < 0.0001);
    assert!(frame.primitives.iter().any(|primitive| {
        primitive.mode == RustyMilkPrimitiveMode::TriangleFan
            && primitive.vertices.iter().any(|value| value.abs() > 0.25)
    }));
}

#[test]
fn rustymilk_expression_vm_samples_fft_data() {
    let mut vars = BTreeMap::new();
    vars.insert(
        "frequency_data".to_string(),
        RustyMilkValue::Text("0,128,255,64".to_string()),
    );
    assert_eq!(
        evaluate_rustymilk_expression("get_fft(0.5)", &vars).unwrap(),
        1.0
    );
    vars.insert(
        "frequency_data".to_string(),
        RustyMilkValue::Text("0,255,0,0".to_string()),
    );
    vars.insert("sample_rate".to_string(), RustyMilkValue::Number(44100.0));
    assert_eq!(
        evaluate_rustymilk_expression("get_fft_hz(5512.5)", &vars).unwrap(),
        1.0
    );
    vars.insert(
        "waveform_data".to_string(),
        RustyMilkValue::Text("-1,0,1,0.5".to_string()),
    );
    assert_eq!(
        evaluate_rustymilk_expression("get_waveform(0.5)", &vars).unwrap(),
        1.0
    );
    assert!(is_rustymilk_function_supported("get_waveform"));
}

#[test]
fn rustymilk_expression_vm_supports_megabuf_state() {
    let scope = evaluate_rustymilk_equations(
        r#"
            q1=2;
            megabuf(q1)=0.25;
            megabuf(q1)+=0.5;
            gmegabuf(4)=megabuf(2)*2;
            q2=megabuf(2)+gmegabuf(4);
            "#,
        &BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(scope.get("megabuf_2"), Some(&RustyMilkValue::Number(0.75)));
    assert_eq!(scope.get("gmegabuf_4"), Some(&RustyMilkValue::Number(1.5)));
    assert_eq!(scope.get("q2"), Some(&RustyMilkValue::Number(2.25)));

    let mut seeded = BTreeMap::new();
    seeded.insert("megabuf_2".to_string(), RustyMilkValue::Number(0.75));
    assert_eq!(
        evaluate_rustymilk_expression("megabuf(2)", &seeded).unwrap(),
        0.75
    );
    assert!(is_rustymilk_function_supported("megabuf"));
    assert!(is_rustymilk_function_supported("gmegabuf"));
}

#[test]
fn rustymilk_rand_varies_by_call_and_frame_scope() {
    let mut first_vars = BTreeMap::new();
    first_vars.insert("frame".to_string(), RustyMilkValue::Number(1.0));
    first_vars.insert("time".to_string(), RustyMilkValue::Number(0.016));
    let first = evaluate_rustymilk_equations("q1=rand(1000); q2=rand(1000);", &first_vars).unwrap();
    let q1 = first.get("q1").and_then(RustyMilkValue::as_number).unwrap();
    let q2 = first.get("q2").and_then(RustyMilkValue::as_number).unwrap();
    assert!((0.0..1000.0).contains(&q1));
    assert!((0.0..1000.0).contains(&q2));
    assert_ne!(q1, q2);

    let mut second_vars = BTreeMap::new();
    second_vars.insert("frame".to_string(), RustyMilkValue::Number(2.0));
    second_vars.insert("time".to_string(), RustyMilkValue::Number(0.032));
    let second = evaluate_rustymilk_equations("q1=rand(1000);", &second_vars).unwrap();
    assert_ne!(
        q1,
        second
            .get("q1")
            .and_then(RustyMilkValue::as_number)
            .unwrap()
    );
}

#[test]
fn rustymilk_per_pixel_equations_build_warp_mesh() {
    let frame = rustymilk_frame_from_source(
        r#"
            name=Warp mesh
            meshx=2
            meshy=1
            zoom=1
            rot=0
            per_pixel_1=zoom=1+rad;
            per_pixel_2=dx=0.1*x;
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );
    let mesh = frame.warp_mesh.expect("warp mesh");
    assert_eq!(mesh.positions.len(), 24);
    assert_eq!(mesh.source_uvs.len(), 24);
    assert_eq!(&mesh.positions[0..6], &[-1.0, -1.0, -1.0, 1.0, 0.0, -1.0]);
    assert!(mesh.source_uvs[0] > 0.1);
    assert!(mesh.source_uvs[0] < 0.25);
    assert!(mesh.source_uvs[2] > 0.1);
    assert!(mesh.source_uvs[4] > 0.45);
}
