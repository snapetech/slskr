use super::*;

#[test]
fn rustymilk_shader_translator_handles_glsl_safe_subset() {
    assert_eq!(
        translate_rustymilk_shader_expression(
            "ret = tex2D(sampler_main, uv).rgb * vec3(0.5, 1.0, 0.25);"
        ),
        "texture(previousFrame, uv).rgb * vec3(0.5, 1.0, 0.25)"
    );
    let shader =
        create_translated_rustymilk_fragment_shader("ret = saturate(vec3(uv.x, uv.y, sin(time)));");
    assert!(shader.contains("uniform sampler2D previousFrame;"));
    assert!(shader.contains("uniform float fftBins[64];"));
    assert!(shader.contains("uniform float waveformBins[64];"));
    assert!(shader.contains("uniform float aspect;"));
    assert!(shader.contains("float rad = length(centeredUv);"));
    assert!(shader.contains("float ang = atan(centeredUv.y, centeredUv.x);"));
    assert!(shader.contains("float get_fft(float position)"));
    assert!(shader.contains("uniform float q64;"));
    assert!(shader.contains("vec3 ret = vec3(clamp01(vec3(uv.x, uv.y, sin(time))));"));
    assert!(analyze_rustymilk_shader_support("ret = vec3(color);").supported);
}

#[test]
fn rustymilk_shader_translator_handles_temps_textures_and_conditionals() {
    let shader = create_translated_rustymilk_fragment_shader(
        r#"
            float2 shifted = uv + float2(frac(time), fmod(time, 1.0));
            float3 tinted = lerp(color, tex2D(sampler_main, shifted).rgb, 0.25);
            float energy = rsqrt(max(get_fft(0.25), 0.001));
            ret = tinted * vec3(energy, atan2(shifted.y, shifted.x), 1.0);
            "#,
    );
    assert!(shader.contains("vec2 shifted = uv + vec2(fract(time), mod(time, 1.0));"));
    assert!(shader.contains("vec3 tinted = mix(color, texture(previousFrame, shifted).rgb, 0.25);"));
    assert!(shader.contains("float energy = inversesqrt(max(get_fft(0.25), 0.001));"));
    assert_eq!(
        translate_rustymilk_shader_expression("float3 tint = vec3(1.0); ret = tint;"),
        "tint"
    );

    assert_eq!(
        get_rustymilk_shader_texture_samplers(
            "ret = tex2D(sampler_noise, uv).rgb + tex2D(album_art, uv).rgb;"
        ),
        vec!["sampler_noise".to_string(), "album_art".to_string()]
    );
    let textured = create_translated_rustymilk_fragment_shader(
        "float3 noise = tex2D(sampler_noise, uv).rgb; ret = noise;",
    );
    assert!(textured.contains("uniform sampler2D shaderTexture0;"));
    assert!(textured.contains("vec3 noise = texture(shaderTexture0, uv).rgb;"));
    let textured_frame = rustymilk_frame_from_source(
        "comp_shader=ret = tex2D(album_art, uv).rgb * tex2D(sampler_noise, uv).rgb;",
        1.0,
        0.1,
        0.2,
        0.3,
    );
    assert_eq!(
        textured_frame.shader_texture_samplers,
        vec!["album_art".to_string(), "sampler_noise".to_string()]
    );

    let conditional = create_translated_rustymilk_fragment_shader(
            "if (bass_att > 1.0) { ret = tex2D(sampler_noise, uv).rgb; } else { ret = vec3(x, y, rad); }",
        );
    assert!(conditional.contains("vec3 ret = vec3((bass_att > 1.0)"));
    assert!(conditional.contains("shaderTexture0"));
    assert!(conditional.contains("vec3(x, y, rad)"));
    assert_eq!(
        translate_rustymilk_shader_expression(
            "if (q1 > 0.5) ret = vec3(1.0); else ret = vec3(0.0);"
        ),
        "(q1 > 0.5) ? (vec3(1.0)) : (vec3(0.0))"
    );
}

#[test]
fn rustymilk_shader_translator_rejects_unsafe_subset() {
    assert_eq!(
        translate_rustymilk_shader_expression("for (;;) { ret = vec3(1.0); }"),
        ""
    );
    assert_eq!(
        translate_rustymilk_shader_expression("ret = unknown[index];"),
        ""
    );
    assert_eq!(
        translate_rustymilk_shader_expression("float3 tint; ret = tint;"),
        ""
    );
    assert!(!analyze_rustymilk_shader_support("if (uv.x > 0.5) ret = vec3(1.0);").supported);
    assert_eq!(
        translate_rustymilk_shader_expression("float3 tint = vec3(1.0); ret = tint; tint *= 0.5;"),
        ""
    );
    assert_eq!(
        translate_rustymilk_shader_expression("ret = vec3(1.0); ret = vec3(0.0);"),
        ""
    );
}

#[test]
fn rustymilk_shader_translator_generates_wgsl_subset() {
    let shader = create_translated_rustymilk_wgsl_shader(
        r#"
            float3 tint = saturate(vec3(q1, bass_att, uv.x));
            tint *= tex2D(sampler_main, uv).rgb;
            ret = tint + vec3(time * 0.01, get_fft(0.25), get_waveform(0.5));
            "#,
    );
    assert!(shader.contains("@fragment"));
    assert!(shader.contains("q64: f32"));
    assert!(shader.contains("fft63: f32"));
    assert!(shader.contains("waveform63: f32"));
    assert!(shader.contains("let q1 = uniforms.q1;"));
    assert!(shader.contains("fn get_fft(position: f32) -> f32"));
    assert!(shader.contains("var tint = clamp01v3(vec3f(q1, bass_att, uv.x));"));
    assert!(shader.contains("tint *= textureSample(previousFrame, previousSampler, uv).rgb;"));
    assert!(shader
        .contains("let ret = vec3f(tint + vec3f(time * 0.01, get_fft(0.25), get_waveform(0.5)));"));

    let textured = create_translated_rustymilk_wgsl_shader("ret = tex2D(sampler_noise, uv).rgb;");
    assert!(textured.contains("@group(0) @binding(3) var shaderTexture0: texture_2d<f32>;"));
    assert!(textured.contains("textureSample(shaderTexture0, shaderTextureSampler, uv).rgb"));
    assert!(
        analyze_rustymilk_webgpu_shader_support("ret = tex2D(sampler_noise, uv).rgb;").supported
    );
    let ternary =
        create_translated_rustymilk_wgsl_shader("ret = q1 > 0.5 ? vec3(1.0) : vec3(0.0);");
    assert!(ternary.contains("let ret = vec3f(select(vec3f(0.0), vec3f(1.0), q1 > 0.5));"));
    assert!(
        analyze_rustymilk_webgpu_shader_support("ret = q1 > 0.5 ? vec3(1.0) : vec3(0.0);")
            .supported
    );
}
