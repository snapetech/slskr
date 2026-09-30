use super::*;

#[test]
fn rustymilk_preset_parser_matches_js_fixture_shapes() {
    let parsed = parse_rustymilk_preset_set(
        r#"
            // comments are ignored
            [preset00]
            name=slskr smoke preset
            fRating=4.0
            fGammaAdj=1.35
            zoom=1.01
            rot=0
            per_frame_1=q1 = bass_att * 0.2;
            per_frame_2=zoom = zoom + q1;
            per_pixel_1=rot = rot + rad * 0.01;
            warp_shader=shader_body {
            warp_shader_1=  ret = texture(sampler_main, uv).xyz;
            warp_shader_2=}
            comp_shader=shader_body { ret = vec3(q1); }
            shape00_enabled=1
            shape00_sides=5
            shape00_init1=q2=0;
            shape00_per_frame1=q2=q2+0.1;
            sprite00_enabled=1
            sprite00_image=logo.png
            sprite00_init1=q3=0.2;
            sprite00_per_frame1=x=0.5+q3;
            wavecode_0_enabled=1
            wavecode_0_samples=512
            wavecode_0_per_point1=x=sample;
            "#,
        false,
    );
    let preset = &parsed.presets[0];
    assert_eq!(parsed.format, "milk");
    assert_eq!(preset.title, "slskr smoke preset");
    assert_eq!(
        preset.base_values.get("fgammaadj"),
        Some(&RustyMilkValue::Number(1.35))
    );
    assert_eq!(
        preset.equations.per_frame,
        "q1 = bass_att * 0.2;\nzoom = zoom + q1;"
    );
    assert_eq!(preset.equations.per_pixel, "rot = rot + rad * 0.01;");
    assert!(preset.warp_shader.contains("texture(sampler_main, uv)"));
    assert_eq!(preset.comp_shader, "shader_body { ret = vec3(q1); }");
    assert_eq!(
        preset.shapes[0].base_values.get("enabled"),
        Some(&RustyMilkValue::Number(1.0))
    );
    assert_eq!(
        preset.shapes[0].base_values.get("sides"),
        Some(&RustyMilkValue::Number(5.0))
    );
    assert_eq!(preset.shapes[0].equations.init, "q2=0;");
    assert_eq!(preset.shapes[0].equations.frame, "q2=q2+0.1;");
    assert_eq!(
        preset.sprites[0].base_values.get("image"),
        Some(&RustyMilkValue::Text("logo.png".to_string()))
    );
    assert_eq!(preset.waves[0].equations.point, "x=sample;");
}

#[test]
fn rustymilk_expression_vm_matches_js_compatibility_cases() {
    let mut vars = BTreeMap::new();
    vars.insert("bass_att".to_string(), RustyMilkValue::Number(2.0));
    assert_eq!(
        evaluate_rustymilk_expression("pow(bass_att, 2) + sqr(3)", &vars).unwrap(),
        13.0
    );
    vars.insert("treb".to_string(), RustyMilkValue::Number(2.0));
    assert_eq!(
        evaluate_rustymilk_expression("if(above(treb, 1.5), sin(0), 7)", &vars).unwrap(),
        0.0
    );
    assert_eq!(
        evaluate_rustymilk_expression("div(10, 0) + sqrt(-1)", &BTreeMap::new()).unwrap(),
        0.0
    );
    assert_eq!(
        evaluate_rustymilk_expression("sin(pi/2)+log(e)+log10(100)", &BTreeMap::new())
            .unwrap()
            .round(),
        4.0
    );
    assert_eq!(
        evaluate_rustymilk_expression("(7 & 3) + (4 | 1) + (7 ^ 3)", &BTreeMap::new()).unwrap(),
        12.0
    );
    assert_eq!(
        evaluate_rustymilk_expression("(1 << 3) + (8 >> 1)", &BTreeMap::new()).unwrap(),
        12.0
    );
    assert_eq!(
        evaluate_rustymilk_expression("~0 + !0 + !2", &BTreeMap::new()).unwrap(),
        0.0
    );
    assert_eq!(
        evaluate_rustymilk_expression(
            "q1 > 0.5 ? q2 + 1 : q3 + 1",
            &BTreeMap::from([
                ("q1".to_string(), RustyMilkValue::Number(1.0)),
                ("q2".to_string(), RustyMilkValue::Number(4.0)),
                ("q3".to_string(), RustyMilkValue::Number(9.0)),
            ])
        )
        .unwrap(),
        5.0
    );
    assert_eq!(
        evaluate_rustymilk_expression(
            "q1 ? 1 : q2 ? 2 : 3",
            &BTreeMap::from([
                ("q1".to_string(), RustyMilkValue::Number(0.0)),
                ("q2".to_string(), RustyMilkValue::Number(1.0)),
            ])
        )
        .unwrap(),
        2.0
    );

    let mut equation_vars = BTreeMap::new();
    equation_vars.insert("bass_att".to_string(), RustyMilkValue::Number(3.0));
    equation_vars.insert("treb_att".to_string(), RustyMilkValue::Number(1.0));
    equation_vars.insert("wave_r".to_string(), RustyMilkValue::Number(0.8));
    equation_vars.insert("zoom".to_string(), RustyMilkValue::Number(1.0));
    let scope = evaluate_rustymilk_equations(
        "q1 = bass_att * 0.2; zoom += q1; q33 = if(below(treb_att, 2), 7, 9); wave_r *= 0.5;",
        &equation_vars,
    )
    .unwrap();
    assert_eq!(
        scope.get("q1"),
        Some(&RustyMilkValue::Number(0.6000000000000001))
    );
    assert_eq!(scope.get("q33"), Some(&RustyMilkValue::Number(7.0)));
    assert_eq!(scope.get("wave_r"), Some(&RustyMilkValue::Number(0.4)));
    let conditional_scope =
        evaluate_rustymilk_equations("q1=0; q2=q1 ? 4 : 8; q3=(q2 == 8) ? 1 : 0;", &equation_vars)
            .unwrap();
    assert_eq!(
        conditional_scope.get("q2"),
        Some(&RustyMilkValue::Number(8.0))
    );
    assert_eq!(
        conditional_scope.get("q3"),
        Some(&RustyMilkValue::Number(1.0))
    );
}

#[test]
fn rustymilk_fragment_parser_and_serializer_match_js_shapes() {
    let fragment = parse_rustymilk_fragment(
        r#"
            [shape]
            sides=7
            rad=0.22
            r=1
            per_frame_1=ang=time;
            "#,
        "star.shape",
        "",
    );
    assert_eq!(fragment.fragment_type, "shape");
    assert_eq!(
        fragment.entries[0].base_values.get("enabled"),
        Some(&RustyMilkValue::Number(1.0))
    );
    assert_eq!(
        fragment.entries[0].base_values.get("sides"),
        Some(&RustyMilkValue::Number(7.0))
    );
    assert_eq!(fragment.entries[0].equations.frame, "ang=time;");
    assert!(serialize_rustymilk_fragment(&fragment.entries[0], "shape")
        .contains("per_frame_1=ang=time;"));

    let wave = parse_rustymilk_fragment(
        "samples=64\nspectrum=1\nper_point_1=x=i;\nper_point_2=y=sample;",
        "spectrum.wave",
        "",
    );
    assert_eq!(wave.fragment_type, "wave");
    assert_eq!(
        wave.entries[0].base_values.get("samples"),
        Some(&RustyMilkValue::Number(64.0))
    );
    assert_eq!(wave.entries[0].equations.point, "x=i;\ny=sample;");

    let prefixed = parse_rustymilk_fragment(
        "shape00_enabled=1\nshape00_sides=4\nshape00_per_frame1=rad=0.25+0.05*sin(time);",
        "prefixed.shape",
        "",
    );
    assert_eq!(
        prefixed.entries[0].base_values.get("sides"),
        Some(&RustyMilkValue::Number(4.0))
    );
    assert_eq!(
        prefixed.entries[0].equations.frame,
        "rad=0.25+0.05*sin(time);"
    );
}

#[test]
fn rustymilk_preset_serializer_keeps_custom_primitives() {
    let parsed = parse_rustymilk_preset_set(
            "name=Serializable\nwave_r=1\nshape00_enabled=1\nshape00_sides=5\nwavecode_0_enabled=1\nwavecode_0_samples=16\nwavecode_0_per_point1=x=i;",
            false,
        );
    let serialized = serialize_rustymilk_preset_set(&parsed);
    assert!(serialized.contains("name=Serializable"));
    assert!(serialized.contains("shape00_sides=5"));
    assert!(serialized.contains("wavecode_0_samples=16"));
    assert!(serialized.contains("wavecode_0_per_point_1=x=i;"));
}

#[test]
fn rustymilk_frame_set_renders_milk2_composite_entries() {
    let frame_set = rustymilk_frame_set_from_source(
        r#"
            [preset00]
            name=Primary
            transition_seconds=2.5
            transition_mode=additive
            wave_r=0.1
            wave_g=0.2
            wave_b=0.3
            shape00_enabled=1
            shape00_sides=3
            [preset01]
            name=Secondary
            blend_alpha=0.35
            composite_mode=screen
            wave_r=0.7
            wave_g=0.6
            wave_b=0.5
            wavecode_0_enabled=1
            "#,
        1.0,
        0.1,
        0.2,
        0.3,
    );

    assert_eq!(frame_set.preset_count, 2);
    assert_eq!(frame_set.title, "Primary + Secondary");
    assert_eq!(frame_set.transition_seconds, 2.5);
    assert_eq!(frame_set.transition_mode, "additive");
    assert_eq!(frame_set.entries[0].blend_alpha, 1.0);
    assert_eq!(frame_set.entries[0].composite_mode, "alpha");
    assert_eq!(frame_set.entries[0].title, "Primary");
    assert_eq!(frame_set.entries[0].frame.shape_count, 1);
    assert_eq!(frame_set.entries[1].blend_alpha, 0.35);
    assert_eq!(frame_set.entries[1].composite_mode, "screen");
    assert_eq!(frame_set.entries[1].title, "Secondary");
    assert_eq!(frame_set.entries[1].frame.waveform_count, 1);
}

#[test]
fn rustymilk_frame_set_runtime_persists_each_preset_state() {
    let mut runtime = RustyMilkFrameSetRuntime::default();
    let source = r#"
            [preset00]
            name=Primary
            q1=0
            per_frame_1=q1=q1+1;
            wave_r=0
            per_frame_2=wave_r=q1/10;
            [preset01]
            name=Secondary
            blend_alpha=0.5
            composite_mode=multiply
            q1=0
            per_frame_1=q1=q1+2;
            wave_g=0
            per_frame_2=wave_g=q1/10;
            shape00_enabled=1
            shape00_init1=q2=0.1;
            shape00_per_frame1=q2=q2+0.1;
            shape00_per_frame2=rad=q2;
            "#;

    let first = runtime.render_source(source, 0.1, 0.1, 0.2, 0.3);
    let second = runtime.render_source(source, 0.2, 0.1, 0.2, 0.3);

    assert_eq!(second.preset_count, 2);
    assert!(second.entries[0].frame.wave_color.0 > first.entries[0].frame.wave_color.0);
    assert!(second.entries[1].frame.wave_color.1 > first.entries[1].frame.wave_color.1);
    assert_eq!(second.entries[1].composite_mode, "multiply");
    assert_eq!(second.entries[1].blend_alpha, 0.5);
    assert!(second.entries[1].frame.primitives.iter().any(|primitive| {
        primitive.mode == RustyMilkPrimitiveMode::TriangleFan
            && primitive.vertices.iter().any(|value| value.abs() > 0.15)
    }));

    let reset = runtime.render_source("name=Reset\nwave_r=0.1", 0.3, 0.1, 0.2, 0.3);
    assert_eq!(reset.preset_count, 1);
}

#[test]
fn rustymilk_validation_reports_secondary_milk2_errors() {
    let error = validate_rustymilk_import(
        r#"
            [preset00]
            name=Primary
            per_frame_1=q1=megabuf(0);
            [preset01]
            name=Secondary
            per_frame_1=q1=unsupported_secondary();
            "#,
    )
    .expect_err("secondary preset should be validated");

    assert!(error
        .contains("preset 2: RustyMilk preset has unsupported functions: unsupported_secondary."));
}

#[test]
fn rustymilk_preset_compatibility_reports_before_rendering() {
    let parsed = parse_rustymilk_preset_set(
        r#"
            per_frame_1=q1=megabuf(0);
            per_pixel_1=q2=sin(pi);
            comp_shader=for (;;) { ret = vec3(1.0); }
            wavecode_0_enabled=1
            wavecode_0_per_point1=y=customcall(sample);
            shape00_enabled=1
            shape00_per_frame1=rad=rand(4);
            sprite00_enabled=1
            sprite00_per_frame1=x=spritecall(time);
            "#,
        false,
    );
    let report = analyze_rustymilk_preset_compatibility(&parsed.presets[0]);
    assert_eq!(
        report.unsupported_functions,
        vec!["customcall".to_string(), "spritecall".to_string()]
    );
    assert_eq!(report.shader_sections, vec!["comp_shader".to_string()]);
    assert_eq!(
            rustymilk_compatibility_error(&report),
            "RustyMilk preset has unsupported functions: customcall, spritecall; shader translation pending: comp_shader."
        );
}

#[test]
fn rustymilk_preset_compatibility_accepts_supported_first_slice() {
    let parsed = parse_rustymilk_preset_set(
        r#"
            per_frame_1=q1=rand(4)+get_fft(0.5)+atan2(1,0);
            per_frame_2=q2=band(7,3)+sigmoid(q1,2);
            warp_shader=ret = tex2D(sampler_main, uv).rgb * vec3(0.5, 0.7, 1.0);
            comp_shader=float2 shifted = uv + float2(frac(time), fmod(time, 1.0));
            comp_shader_1=float energy = rsqrt(max(get_fft(0.5), 0.001));
            comp_shader_2=ret = vec3(shifted, energy * bass_att);
            "#,
        false,
    );
    let report = analyze_rustymilk_preset_compatibility(&parsed.presets[0]);
    assert!(report.unsupported_functions.is_empty());
    assert!(report.shader_sections.is_empty());
    assert_eq!(rustymilk_compatibility_error(&report), "");
}

#[test]
fn rustymilk_import_validation_accepts_only_supported_presets() {
    assert_eq!(
        validate_rustymilk_import(
            "name=Imported\nper_frame_1=q1=rand(4)+get_fft(0.5);\ncomp_shader=ret = vec3(q1);"
        ),
        Ok("Imported".to_string())
    );
    let error = validate_rustymilk_import(
            "name=Bad\nper_frame_1=q1=megabuf(0);\nper_frame_2=q2=unknowncall(q1);\ncomp_shader=for (;;) { ret = vec3(1.0); }",
        )
        .expect_err("unsupported preset should be rejected");
    assert!(error.contains("unsupported functions: unknowncall"));
    assert!(error.contains("shader translation pending: comp_shader"));
}

#[test]
fn rustymilk_compatibility_matrix_tracks_dense_primitive_pressure() {
    let mut lines = vec!["name=Dense Compatibility Probe".to_string()];
    for index in 0..40 {
        lines.push(format!("shape{index:02}_enabled=1"));
        lines.push(format!("shape{index:02}_sides=5"));
        lines.push(format!("shape{index:02}_rad=0.1"));
    }
    for index in 0..20 {
        lines.push(format!("wavecode_{index}_enabled=1"));
        lines.push(format!("wavecode_{index}_samples=16"));
        lines.push(format!("wavecode_{index}_per_point1=x=i;"));
    }
    let entry =
        build_rustymilk_compatibility_entry("dense-pack-probe", "", &lines.join("\n"), false);
    assert!(entry.supported);
    assert!(entry.webgpu_supported);
    assert_eq!(entry.metrics.max_shape_count, 40);
    assert_eq!(entry.metrics.max_wave_count, 20);
    let summary = summarize_rustymilk_compatibility_matrix(&[entry]);
    assert_eq!(summary.total_count, 1);
    assert_eq!(summary.supported_count, 1);
    assert_eq!(summary.max_shape_count, 40);
    assert_eq!(summary.max_wave_count, 20);
}

#[test]
fn rustymilk_compatibility_matrix_tracks_q_register_and_webgpu_gaps() {
    let entry = build_rustymilk_compatibility_entry(
        "q-register-pack-probe",
        "",
        r#"
            [preset00]
            q64=0.5
            per_frame_1=q1=q64+bass;
            wavecode_0_enabled=1
            wavecode_0_per_point1=y=q48+sample;
            [preset01]
            per_frame_1=q63=q1+treb;
            shape00_enabled=1
            shape00_per_frame1=q32=q63*0.5;
            comp_shader=ret = tex2D(album_art, uv).rgb * vec3(q32, get_fft(0.5), get_waveform(0.5));
            "#,
        true,
    );
    assert!(entry.supported);
    assert!(entry.webgpu_supported);
    assert!(entry.webgpu_shader_sections.is_empty());
    assert_eq!(entry.preset_count, 2);
    assert_eq!(entry.metrics.max_q_register_index, 64);
    assert_eq!(
        entry.metrics.q_registers,
        vec![
            "q1".to_string(),
            "q32".to_string(),
            "q48".to_string(),
            "q63".to_string(),
            "q64".to_string()
        ]
    );

    let webgpu_ternary = build_rustymilk_compatibility_entry(
        "webgpu-shader-ternary-probe",
        "",
        "comp_shader=ret = q1 > 0.5 ? vec3(1.0) : vec3(0.0);",
        false,
    );
    assert!(webgpu_ternary.supported);
    assert!(webgpu_ternary.webgpu_supported);
    assert!(webgpu_ternary.webgpu_shader_sections.is_empty());

    let webgpu_gap = build_rustymilk_compatibility_entry(
        "webgpu-shader-gap-probe",
        "",
        "comp_shader=ret = (q1 & 3) > 0 ? vec3(1.0) : vec3(0.0);",
        false,
    );
    assert!(webgpu_gap.supported);
    assert!(!webgpu_gap.webgpu_supported);
    assert_eq!(
        webgpu_gap.webgpu_shader_sections,
        vec!["comp_shader".to_string()]
    );
    let summary = summarize_rustymilk_compatibility_matrix(&[entry, webgpu_ternary, webgpu_gap]);
    assert_eq!(summary.total_count, 3);
    assert_eq!(summary.webgpu_supported_count, 2);
    assert_eq!(summary.webgpu_unsupported_count, 1);
    assert_eq!(
        summary.webgpu_unsupported_shader_sections,
        vec!["comp_shader".to_string()]
    );
}
