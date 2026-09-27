#[cfg(test)]
mod tests {
    use super::*;

    const REACT_ROUTES: &str = include_str!("../../../web/src/components/AppRouteTable.jsx");
    const REACT_NAV: &str = include_str!("../../../web/src/components/AppNavigationPrimary.jsx");
    const REACT_HEADER: &str = include_str!("../../../web/src/components/AppHeaderMenu.jsx");
    const STATIC_INDEX: &str = include_str!("../static/index.html");

    fn rounded_vec(values: &[f64]) -> Vec<f64> {
        values
            .iter()
            .map(|value| (value * 1000.0).round() / 1000.0)
            .collect()
    }

    fn populated_route_html(path: &str) -> String {
        let (endpoint_path, body) = match route_kind(path) {
            RouteKind::Search | RouteKind::DiscoveryGraph => (
                "/searches/:id/responses",
                r#"[{"id":"search-1","username":"peer1","files":[{"filename":"Archive/Track.flac"}],"queueLength":1,"hasFreeUploadSlot":true}]"#,
            ),
            RouteKind::PlaylistIntake => (
                "/source-feed-imports/preview",
                r#"[{"artist":"Archive Artist","title":"Public Domain Theme","status":"Matched"}]"#,
            ),
            RouteKind::Wishlist => (
                "/wishlist",
                r#"[{"id":"wish-1","searchText":"rare live set","filter":"flac","enabled":true,"autoDownload":false}]"#,
            ),
            RouteKind::Downloads => (
                "/transfers/downloads",
                r#"[{"id":77,"username":"peer1","filename":"Remote/Song.mp3","state":"Queued","progress":0.5}]"#,
            ),
            RouteKind::Uploads => (
                "/transfers/uploads",
                r#"[{"id":78,"username":"peer1","filename":"Remote/Upload.mp3","state":"Queued","progress":0.5}]"#,
            ),
            RouteKind::Messages | RouteKind::Rooms => (
                "/conversations",
                r#"[{"username":"peer1","lastMessage":"hello","unreadCount":1}]"#,
            ),
            RouteKind::Users => (
                "/users",
                r#"[{"username":"peer1","status":"Online","files":2}]"#,
            ),
            RouteKind::Contacts => (
                "/contacts",
                r#"[{"nickname":"Peer One","peerId":"peer1","group":"trusted","verified":true}]"#,
            ),
            RouteKind::Solid => (
                "/solid/status",
                r#"{"webId":"https://example.test/profile#me","storage":"ready","status":"connected"}"#,
            ),
            RouteKind::Collections => (
                "/collections",
                r#"[{"id":"collection-1","title":"Fixture Collection","type":"ShareList","itemCount":1}]"#,
            ),
            RouteKind::ShareGroups => (
                "/sharegroups",
                r#"[{"id":"group-1","name":"Fixture Group","memberCount":1,"createdAt":"today"}]"#,
            ),
            RouteKind::SharedWithMe => (
                "/share-grants",
                r#"[{"id":"grant-1","title":"Fixture Share","owner":"peer1","permissions":"read"}]"#,
            ),
            RouteKind::Browse => (
                "/users/:username/browse",
                r#"[{"name":"/Music/Open Sessions","isDirectory":true,"size":0},{"name":"/Music/Open Sessions/Track.flac","isDirectory":false,"size":1234}]"#,
            ),
            RouteKind::System => (
                "/server",
                r#"{"state":"connected","username":"audit-user"}"#,
            ),
        };
        route_workspace_result_html(
            path,
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: endpoint_path,
                    surface: "test",
                },
                body: body.to_string(),
            }],
        )
    }

    #[test]
    fn api_endpoints_are_versioned() {
        for section in app_sections() {
            assert!(endpoint_url(section.endpoint).starts_with("/api/v0/"));
        }
    }

    #[test]
    fn shell_contains_primary_routes() {
        let html = shell_html();
        for item in nav_items() {
            assert!(html.contains(item.label), "missing {}", item.label);
        }
        assert!(html.contains("Search, transfers, messages"));
        assert!(html.contains("donations%40snape.tech"));
        assert!(html.contains("https://ko-fi.com/snapetech"));
        assert!(html.contains("slskr-player"));
        assert!(html.contains("data-slskr-player"));
        assert!(html.contains("slskr-player-audio"));
        assert!(html.contains("data-slskr-player-action=\"play\""));
        assert!(html.contains("data-slskr-player-action=\"refresh\""));
        assert!(html.contains("data-slskr-player-action=\"clear\""));
        assert!(html.contains("data-slskr-player-action=\"visualizer\""));
        assert!(html.contains("data-slskr-player-action=\"radio\""));
        assert!(html.contains("data-slskr-player-radio-query"));
        assert!(html.contains("data-slskr-player-rating=\"5\""));
        assert!(html.contains("slskr-player-rating-status"));
        assert!(html.contains("slskr-player-radio"));
        assert!(html.contains("Searches"));
        assert!(html.contains("Search Detail"));
        assert!(html.contains("data-slskr-route-kind=\"Search\""));
        assert!(html.contains("slskr-player-now"));
        assert!(html.contains("slskr-player-transfers"));
        assert!(html.contains("slskr-rust-rustymilk"));
        assert!(html.contains("data-slskr-rustymilk-search=\"\""));
        assert!(html.contains("data-slskr-rustymilk-playlist=\"\""));
        assert!(html.contains("data-slskr-rustymilk-automation=\"off\""));
        assert!(html.contains("data-slskr-rustymilk-fps=\"full\""));
        assert!(html.contains("data-slskr-rustymilk-quality=\"balanced\""));
        assert!(html.contains("slskr-rustymilk-canvas"));
        assert!(html.contains("Search RustyMilk presets"));
        assert!(html.contains("data-slskr-rustymilk-action=\"previous\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"preset\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"random\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"favorite\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"favorites\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"remove\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"search\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"clear-search\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"save-playlist\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"playlist\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"rename-playlist\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"clear-playlist\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"remove-playlist\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"import\""));
        assert!(html.contains("slskr-rustymilk-preset-input"));
        assert!(html.contains("accept=\".milk,.milk2,.txt,text/plain\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"pack\""));
        assert!(html.contains("slskr-rustymilk-pack-input"));
        assert!(html.contains("webkitdirectory hidden"));
        assert!(html.contains("data-slskr-rustymilk-action=\"texture\""));
        assert!(html.contains("slskr-rustymilk-texture-input"));
        assert!(html.contains("multiple hidden"));
        assert!(html.contains("slskr-rustymilk-textures"));
        assert!(html.contains("data-slskr-rustymilk-action=\"clear\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"reset\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"apply-parameter\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"randomize-parameters\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"import-shape\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"export-shape\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"remove-shape\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"import-wave\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"export-wave\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"remove-wave\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"export-preset\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"automation\""));
        assert!(html.contains("data-slskr-rustymilk-action=\"debug\""));
        assert!(html.contains("slskr-rustymilk-debug"));
        assert!(html.contains("slskr-rustymilk-renderer"));
        assert!(html.contains("/api/v0/searches"));
        assert!(html.contains("slskr-runtime-status"));
        assert!(html.contains("/api/v0/health"));
        assert!(html.contains("slskr-route-view"));
        let system = route_page_html("/system");
        assert!(system.contains("System"));
        assert!(system.contains("Rescan Shares"));
    }

    #[test]
    fn rust_player_surface_summarizes_live_payloads() {
        let (title, detail) = player_now_playing_text(
            r#"{"now_playing":[{"artist":"Archive Artist","album":"Open Sessions","title":"Public Domain Theme"}]}"#,
        );
        assert_eq!(title, "Public Domain Theme");
        assert_eq!(detail, "Archive Artist / Open Sessions");
        assert_eq!(
            player_transfer_text(r#"{"downloads":2,"uploads":1}"#),
            "2 down / 1 up"
        );
        assert_eq!(
            player_party_text(r#"{"count":3,"active_parties":[]}"#),
            "3 listening parties"
        );
        assert_eq!(
            player_visualizer_text(r#"{"status":"configured","configured":true}"#),
            "configured"
        );
        let track = serde_json::json!({
            "artist": "Archive Artist",
            "album": "Open Sessions",
            "title": "Public Domain Theme"
        });
        assert_eq!(
            player_rating_key(&track),
            "meta:archive artist|open sessions|public domain theme"
        );
        assert_eq!(
            player_stream_url(&serde_json::json!({"contentId":"sha256:track"})),
            "/api/v0/streams/sha256%3Atrack"
        );
        assert_eq!(
            player_stream_url(&serde_json::json!({"streamUrl":"/custom/stream"})),
            "/custom/stream"
        );
        assert_eq!(player_rating_summary(1), "Discovery caution");
        assert_eq!(player_rating_summary(3), "Neutral rating");
        assert_eq!(player_rating_summary(5), "Discovery boost");
        assert_eq!(player_rating_summary(0), "Not rated");
        let radio_plan = build_player_radio_plan(Some(&track));
        assert!(radio_plan.ready);
        assert_eq!(
            radio_plan.seed_label,
            "Archive Artist - Public Domain Theme"
        );
        assert_eq!(
            radio_plan.queries,
            vec![
                PlayerRadioQuery {
                    id: "radio-query-1".to_string(),
                    query: "Archive Artist Public Domain Theme".to_string(),
                    reason: "Similar track seed",
                },
                PlayerRadioQuery {
                    id: "radio-query-2".to_string(),
                    query: "Archive Artist Open Sessions".to_string(),
                    reason: "Album neighborhood",
                },
                PlayerRadioQuery {
                    id: "radio-query-3".to_string(),
                    query: "Archive Artist".to_string(),
                    reason: "Artist and genre seed",
                },
            ]
        );
        assert_eq!(
            build_player_radio_search_path(&radio_plan.primary_query),
            "/searches?q=Archive%20Artist%20Public%20Domain%20Theme"
        );
        assert_eq!(
            player_radio_queries(&radio_plan, 2),
            vec![
                "Archive Artist Public Domain Theme".to_string(),
                "Archive Artist Open Sessions".to_string(),
            ]
        );
        assert!(player_radio_copy_text(&radio_plan)
            .contains("Similar track seed: \"Archive Artist Public Domain Theme\""));
        assert_eq!(
            player_radio_query_from_now_playing_body(
                r#"{"track":{"artist":"Archive Artist","title":"Public Domain Theme"}}"#
            ),
            "Archive Artist Public Domain Theme"
        );
        assert_eq!(
            build_player_radio_plan(None),
            PlayerRadioPlan {
                basis: Vec::new(),
                primary_query: String::new(),
                queries: Vec::new(),
                ready: false,
                seed_label: "No track selected".to_string(),
            }
        );
    }

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
        let conditional_scope = evaluate_rustymilk_equations(
            "q1=0; q2=q1 ? 4 : 8; q3=(q2 == 8) ? 1 : 0;",
            &equation_vars,
        )
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
        let first =
            evaluate_rustymilk_equations("q1=rand(1000); q2=rand(1000);", &first_vars).unwrap();
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
    fn rustymilk_shader_translator_handles_glsl_safe_subset() {
        assert_eq!(
            translate_rustymilk_shader_expression(
                "ret = tex2D(sampler_main, uv).rgb * vec3(0.5, 1.0, 0.25);"
            ),
            "texture(previousFrame, uv).rgb * vec3(0.5, 1.0, 0.25)"
        );
        let shader = create_translated_rustymilk_fragment_shader(
            "ret = saturate(vec3(uv.x, uv.y, sin(time)));",
        );
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
        assert!(
            shader.contains("vec3 tinted = mix(color, texture(previousFrame, shifted).rgb, 0.25);")
        );
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
            translate_rustymilk_shader_expression(
                "float3 tint = vec3(1.0); ret = tint; tint *= 0.5;"
            ),
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
        assert!(shader.contains(
            "let ret = vec3f(tint + vec3f(time * 0.01, get_fft(0.25), get_waveform(0.5)));"
        ));

        let textured =
            create_translated_rustymilk_wgsl_shader("ret = tex2D(sampler_noise, uv).rgb;");
        assert!(textured.contains("@group(0) @binding(3) var shaderTexture0: texture_2d<f32>;"));
        assert!(textured.contains("textureSample(shaderTexture0, shaderTextureSampler, uv).rgb"));
        assert!(
            analyze_rustymilk_webgpu_shader_support("ret = tex2D(sampler_noise, uv).rgb;")
                .supported
        );
        let ternary =
            create_translated_rustymilk_wgsl_shader("ret = q1 > 0.5 ? vec3(1.0) : vec3(0.0);");
        assert!(ternary.contains("let ret = vec3f(select(vec3f(0.0), vec3f(1.0), q1 > 0.5));"));
        assert!(
            analyze_rustymilk_webgpu_shader_support("ret = q1 > 0.5 ? vec3(1.0) : vec3(0.0);")
                .supported
        );
    }

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
    fn rustymilk_webgpu_vertex_packers_match_js_shapes() {
        assert_eq!(
            rounded_vec(&create_rustymilk_webgpu_triangle_list_vertices(
                &[-1.0, -1.0, 1.0, -1.0, 0.0, 1.0],
                [0.2, 0.4, 0.6, 0.8],
            )),
            vec![
                -1.0, -1.0, 0.2, 0.4, 0.6, 0.8, 1.0, -1.0, 0.2, 0.4, 0.6, 0.8, 0.0, 1.0, 0.2, 0.4,
                0.6, 0.8,
            ]
        );
        assert_eq!(
            rounded_vec(&create_rustymilk_webgpu_triangle_fan_vertices(
                &[0.0, 0.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0],
                &[1.0, 0.0, 0.0, 0.5, 0.0, 1.0, 0.0, 0.6, 0.0, 0.0, 1.0, 0.7, 1.0, 1.0, 1.0, 0.8,],
                [1.0, 1.0, 1.0, 1.0],
            )),
            vec![
                0.0, 0.0, 1.0, 0.0, 0.0, 0.5, -1.0, -1.0, 0.0, 1.0, 0.0, 0.6, 1.0, -1.0, 0.0, 0.0,
                1.0, 0.7, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, -1.0, 0.0, 0.0, 1.0, 0.7, 1.0, 1.0,
                1.0, 1.0, 1.0, 0.8,
            ]
        );
        assert_eq!(
            rounded_vec(&create_rustymilk_webgpu_line_segment_vertices(
                &[-1.0, 0.0, 0.0, 0.5, 1.0, 0.0],
                [0.1, 0.2, 0.3, 0.4],
            )),
            vec![
                -1.0, 0.0, 0.1, 0.2, 0.3, 0.4, 0.0, 0.5, 0.1, 0.2, 0.3, 0.4, 0.0, 0.5, 0.1, 0.2,
                0.3, 0.4, 1.0, 0.0, 0.1, 0.2, 0.3, 0.4,
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
                0.0, 0.0, 0.5, 0.5, 1.0, 0.0, 0.0, 0.5, -1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.6,
                1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.7,
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
            batches.textured_batches[batches.composite_batches[1].textured_batch_first]
                .first_vertex,
            batches.composite_batches[1].textured_first_vertex
        );
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

        assert!(error.contains(
            "preset 2: RustyMilk preset has unsupported functions: unsupported_secondary."
        ));
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
        let summary =
            summarize_rustymilk_compatibility_matrix(&[entry, webgpu_ternary, webgpu_gap]);
        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.webgpu_supported_count, 2);
        assert_eq!(summary.webgpu_unsupported_count, 1);
        assert_eq!(
            summary.webgpu_unsupported_shader_sections,
            vec!["comp_shader".to_string()]
        );
    }

    #[test]
    fn rust_player_surface_builds_similar_queue_candidates() {
        let current = serde_json::json!({
            "album": "Fixture Album",
            "artist": "Fixture Artist",
            "contentId": "sha256:current",
            "genre": "Fixture Genre",
            "title": "Current Song"
        });
        let history = vec![
            serde_json::json!({
                "album": "Fixture Album",
                "artist": "Fixture Artist",
                "contentId": "sha256:album-match",
                "title": "Album Match"
            }),
            serde_json::json!({
                "artist": "Other Artist",
                "contentId": "sha256:tag-match",
                "tags": ["Fixture Genre"],
                "title": "Tag Match"
            }),
            serde_json::json!({
                "artist": "Other Artist",
                "contentId": "sha256:miss",
                "title": "Miss"
            }),
        ];
        let queue = vec![serde_json::json!({"contentId": "sha256:current"})];
        let candidates = build_similar_queue_candidates(Some(&current), &history, &queue, 5);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.item["contentId"].as_str().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["sha256:album-match", "sha256:tag-match"]
        );
        assert_eq!(
            similar_queue_search_queries(&candidates, 3),
            vec![
                "Fixture Artist Album Match".to_string(),
                "Other Artist Tag Match".to_string(),
            ]
        );

        let duplicate_history = vec![
            serde_json::json!({
                "artist": "Fixture Artist",
                "contentId": "sha256:queued",
                "title": "Already Queued"
            }),
            serde_json::json!({
                "artist": "Fixture Artist",
                "contentId": "sha256:new",
                "title": "New Candidate"
            }),
            serde_json::json!({
                "artist": "Fixture Artist",
                "contentId": "sha256:new",
                "title": "Duplicate Candidate"
            }),
        ];
        let duplicate_queue = vec![
            serde_json::json!({"contentId": "sha256:current"}),
            serde_json::json!({"contentId": "sha256:queued"}),
        ];
        let candidates =
            build_similar_queue_candidates(Some(&current), &duplicate_history, &duplicate_queue, 5);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.item["contentId"].as_str().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["sha256:new"]
        );
        assert!(build_similar_queue_candidates(None, &history, &queue, 5).is_empty());
    }

    #[test]
    fn rust_browser_local_system_panels_match_react_surfaces() {
        assert_eq!(experience_preferences().len(), 18);
        let defaults = default_experience_preferences();
        assert_eq!(
            defaults
                .get("searchRankingProfile")
                .and_then(|value| value.as_str()),
            Some("balanced")
        );
        assert_eq!(
            defaults
                .get("playerKeyboardShortcuts")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        let report = experience_preferences_report(&defaults);
        assert!(report.contains("slskr experience preferences"));
        assert!(report.contains("Player: queue_auto_fill=false"));
        assert!(
            experience_settings_panel_html().contains("data-slskr-pref=\"playerRadioSeedMode\"")
        );

        assert_eq!(automation_recipes().len(), 7);
        let mut state = serde_json::Map::new();
        state.insert(
            "wishlist-retry".to_string(),
            serde_json::json!({
                "enabled": true,
                "lastDryRunAt": "browser-local"
            }),
        );
        let (total, enabled, disabled) = automation_summary_from_state(&state);
        assert_eq!((total, enabled, disabled), (7, 4, 3));
        let dry_run = automation_dry_run_report(automation_recipes()[3], "browser-local");
        assert_eq!(dry_run["recipeId"], "wishlist-retry");
        assert_eq!(dry_run["executed"], false);
        let history = automation_history_report(&state);
        assert!(history.contains("slskr automation review history"));
        assert!(history.contains("Wishlist Retry"));
        assert!(
            automation_center_panel_html().contains("data-slskr-recipe=\"library-health-scan\"")
        );
    }

    #[test]
    fn rust_search_planner_matches_react_search_helpers() {
        let strong = serde_json::json!({
            "files": [{
                "bitDepth": 16,
                "filename": "Boards of Canada/Music Has The Right/01 Wildlife Analysis.flac",
                "sampleRate": 44100,
                "size": 24000000
            }],
            "hasFreeUploadSlot": true,
            "queueLength": 0,
            "uploadSpeed": 4000000,
            "username": "good-peer"
        });
        let rank = rank_search_candidate(
            &strong,
            "boards canada wildlife analysis",
            "lossless-exact",
            Some(&serde_json::json!({"successfulDownloads": 4, "failedDownloads": 0})),
            None,
            None,
        );
        assert!(rank.score >= 80);
        assert!(rank.reasons.contains(&"strong filename match".to_string()));
        assert!(rank.reasons.contains(&"mostly lossless files".to_string()));
        assert!(rank.reasons.contains(&"free upload slot".to_string()));

        let weak = serde_json::json!({
            "files": [{"bitRate": 128, "filename": "misc/upload/track.mp3", "size": 2000000}],
            "hasFreeUploadSlot": false,
            "queueLength": 8,
            "uploadSpeed": 64000,
            "username": "rough-peer"
        });
        let weak_rank = rank_search_candidate(
            &weak,
            "boards canada wildlife analysis",
            "lossless-exact",
            Some(&serde_json::json!({"successfulDownloads": 0, "failedDownloads": 4})),
            None,
            None,
        );
        assert!(weak_rank.score < 35);
        assert!(weak_rank.reasons.contains(&"long queue".to_string()));
        assert!(weak_rank
            .reasons
            .contains(&"poor download history".to_string()));

        let fast = serde_json::json!({
            "files": [{"bitRate": 320, "filename": "Stereolab/Peng!/Super Falling Star.mp3", "size": 8000000}],
            "hasFreeUploadSlot": true,
            "queueLength": 1,
            "uploadSpeed": 1000000
        });
        let fast_rank = rank_search_candidate(
            &fast,
            "stereolab super falling star",
            "fast-good-enough",
            None,
            None,
            None,
        );
        assert!(fast_rank.score >= 60);
        assert!(fast_rank
            .reasons
            .contains(&"high bitrate fast-good-enough candidate".to_string()));

        let responses = vec![
            serde_json::json!({
                "files": [{"filename": "Artist/Album/01 Track.flac", "size": 24000000}],
                "primarySource": "mesh",
                "sourceProviders": ["mesh"],
                "username": "best-peer"
            }),
            serde_json::json!({
                "files": [{"filename": "Different Root/01 Track.flac", "size": 24000000}],
                "primarySource": "soulseek",
                "sourceProviders": ["soulseek"],
                "username": "backup-peer"
            }),
            serde_json::json!({
                "files": [{"filename": "Artist/Album/02 Other.flac", "size": 22000000}],
                "username": "other-peer"
            }),
        ];
        let (folded, groups) = deduplicate_search_response_groups(&responses, true);
        assert_eq!(folded, 1);
        assert_eq!(groups[0].candidate_count, 2);
        assert_eq!(groups[0].folded_count, 1);
        assert_eq!(
            groups[0].usernames,
            vec!["backup-peer".to_string(), "best-peer".to_string()]
        );

        let preview = build_search_action_preview(
            &serde_json::json!({
                "hasFreeUploadSlot": false,
                "queueLength": 7,
                "sourceProviders": ["pod", "scene"],
                "username": "peer"
            }),
            &[
                serde_json::json!({"filename": "Artist/Album/01 Track.flac", "size": 20}),
                serde_json::json!({"filename": "Artist/Album/02 Track.flac", "locked": true, "size": 30}),
            ],
            Some(&SearchCandidateRank {
                reasons: Vec::new(),
                score: 38,
            }),
            Some(&serde_json::json!({
                "override": {"mode": "ignore", "note": "Known private peer."},
                "score": -6
            })),
            "download",
        );
        assert_eq!(preview.file_count, 2);
        assert_eq!(preview.locked_count, 1);
        assert!(preview
            .warnings
            .contains(&"No free upload slot is currently advertised".to_string()));
        let text = format_search_action_preview(&preview);
        assert!(text.contains("Action: download"));
        assert!(text.contains("Candidate score: 38/100"));
        assert!(
            search_planner_report("public domain theme", "lossless-exact", true)
                .contains("Search planner")
        );
    }

    #[test]
    fn static_index_supports_direct_nested_route_loads() {
        assert!(STATIC_INDEX.contains("href=\"/styles.css\""));
        assert!(STATIC_INDEX.contains("src=\"/slskr_web_bootstrap.js\""));
        assert!(!STATIC_INDEX.contains("href=\"./styles.css\""));
        assert!(!STATIC_INDEX.contains("src=\"./slskr_web_bootstrap.js\""));
    }

    #[test]
    fn runtime_probe_html_escapes_api_values() {
        let html = runtime_probe_result_html(&[(
            "Probe",
            "/api/v0/probe",
            Ok(r#"<script>"bad"</script>"#),
        )]);
        assert!(html.contains("&lt;script&gt;&quot;bad&quot;&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn runtime_probes_cover_public_and_session_status() {
        let paths = runtime_probes()
            .iter()
            .map(|probe| probe.path)
            .collect::<Vec<_>>();
        for expected in ["/health", "/version", "/application", "/server"] {
            assert!(
                paths.contains(&expected),
                "missing runtime probe {expected}"
            );
        }
    }

    #[test]
    fn rust_route_pages_cover_current_route_inventory() {
        let pages = route_pages()
            .iter()
            .map(|page| page.path)
            .collect::<Vec<_>>();
        for route in ui_routes() {
            if route.path == "/" {
                continue;
            }
            assert!(
                pages.contains(&route.path),
                "missing route page for {}",
                route.path
            );
        }
    }

    #[test]
    fn route_normalization_handles_dynamic_routes() {
        assert_eq!(normalize_route_path("/"), "/searches");
        assert_eq!(normalize_route_path("/searches/42"), "/searches/:id");
        assert_eq!(normalize_route_path("/system/network"), "/system/:tab");
        assert_eq!(normalize_route_path("/pods/abc"), "/pods/:podId");
        assert_eq!(
            normalize_route_path("/pods/abc/channels/general"),
            "/pods/:podId/channels/:channelId"
        );
    }

    #[test]
    fn route_pages_render_api_surface() {
        let html = route_page_html("/downloads");
        assert!(html.contains("Downloads"));
        assert!(html.contains("/api/v0/transfers/downloads"));
        assert!(html.contains("data-route=\"/downloads\""));
        assert!(html.contains("data-slskr-refresh-scope=\"active-route\""));
        assert!(html.contains("data-slskr-lazy-workspace=\"true\""));
        assert!(html.contains("data-slskr-lazy-diagnostics=\"true\""));
        assert!(html.contains("slskr-route-data"));
        assert!(html.contains("Workspace"));
        assert!(html.contains("<h3>Downloads</h3>"));
        assert!(html.contains("slskr-route-actions"));
        assert!(html.contains("slskr-route-summary"));
        assert!(html.contains("Overview"));
        assert!(html.contains("slskr-page-data"));
        assert!(html.contains("Developer"));
        assert!(html.contains("Clear Completed Downloads"));
        assert!(html.contains("data-slskr-refresh-route"));
        assert!(html.contains("data-slskr-focus-filter"));
        assert!(html.contains("data-slskr-clear-filters"));
        assert!(html.contains("slskr-live-status"));
    }

    #[test]
    fn active_route_refresh_contract_keeps_hidden_panes_lazy() {
        let shell = shell_html();
        assert_eq!(
            shell
                .matches("data-slskr-refresh-scope=\"active-route\"")
                .count(),
            1,
            "the shell should render one active route instead of pre-rendering every route pane"
        );
        assert_eq!(
            shell.matches("id=\"slskr-route-data\"").count(),
            1,
            "only the active route should own live probe status"
        );
        assert_eq!(
            shell.matches("id=\"slskr-page-data\"").count(),
            1,
            "only the active route should own live workspace data"
        );
        assert!(shell.contains("data-slskr-lazy-workspace=\"true\""));
        assert!(shell.contains("data-slskr-lazy-diagnostics=\"true\""));
        assert!(shell.contains("id=\"slskr-rust-rustymilk\" hidden"));
        assert!(shell.contains("data-slskr-rustymilk-running=\"false\""));
        assert!(
            !shell.contains("setInterval("),
            "initial shell markup should not register polling loops for hidden panes"
        );

        for path in ["/searches", "/downloads", "/messages", "/browse", "/system"] {
            let page = route_page_html(path);
            assert!(
                page.contains("data-slskr-live-state=\"pending\""),
                "{path} should render pending live data until the active route refresh runs"
            );
        }
    }

    #[test]
    fn route_pages_render_domain_workflows_before_developer_details() {
        let expectations = [
            ("/searches", "Searches", "Search", "search-native"),
            (
                "/discovery-graph",
                "Discovery Graph Atlas",
                "Build graph",
                "discovery-graph-native",
            ),
            (
                "/playlist-intake",
                "Playlist Intake",
                "Preview playlist",
                "playlist-intake-native",
            ),
            (
                "/wishlist",
                "Wishlist",
                "Add wanted search",
                "wishlist-native",
            ),
            ("/downloads", "Downloads", "Download", "transfers-native"),
            ("/uploads", "Uploads", "Clear completed", "transfers-native"),
            ("/messages", "Conversations", "Reply", "messaging-native"),
            ("/users", "Users", "Watch", "users-native"),
            ("/contacts", "Contacts", "Add contact", "contacts-native"),
            ("/solid", "Solid", "Connect identity", "solid-native"),
            (
                "/collections",
                "Collections",
                "Create Collection",
                "collections-native",
            ),
            (
                "/sharegroups",
                "Share Groups",
                "Issue token",
                "sharegroups-native",
            ),
            ("/shared", "Shared with Me", "Open", "shared-native"),
            ("/browse", "Browse", "Browse", "browse-native"),
            ("/system", "System", "Rescan Shares", "system-native"),
        ];

        for (path, heading, action, native_class) in expectations {
            let html = route_page_html(path);
            let heading_index = html
                .find(heading)
                .unwrap_or_else(|| panic!("missing workflow heading {heading} for route {path}"));
            let developer_index = html
                .find("<summary>Developer</summary>")
                .unwrap_or_else(|| panic!("missing developer drawer for route {path}"));

            assert!(
                heading_index < developer_index,
                "route {path} should show workflow content before developer diagnostics"
            );
            assert!(
                html.contains(action),
                "missing primary action {action} for route {path}"
            );
            assert!(html.contains("slskr-workflow"));
            assert!(html.contains("slskr-native-workspace"));
            assert!(html.contains("slskr-native-subviews"));
            assert!(html.contains("slskr-native-panel-actions"));
            assert!(html.contains("slskr-native-panel-fields"));
            assert!(html.contains("slskr-native-panel-facts"));
            assert!(html.contains("data-slskr-native-tab=\"0\""));
            assert!(html.contains("data-slskr-native-panel=\"0\""));
            assert!(html.contains("data-slskr-native-filter"));
            assert!(html.contains("data-slskr-native-count"));
            assert!(html.contains("data-slskr-native-select-visible"));
            assert!(html.contains("data-slskr-native-clear-selection"));
            assert!(html.contains("data-slskr-native-reset-state"));
            if html.contains("aria-keyshortcuts") {
                assert!(
                    html.contains("aria-keyshortcuts=\"Enter Space ArrowUp ArrowDown Home End\"")
                );
                assert!(html.contains("data-slskr-native-sort=\"0\""));
                assert!(html.contains("data-slskr-native-sort-0="));
                assert!(html.contains("data-slskr-native-index="));
            } else {
                assert!(html.contains("slskr-native-empty"));
            }
            assert!(html.contains("slskr-native-inspector"));
            assert!(html.contains("data-slskr-native-inspector-title"));
            if html.contains("aria-keyshortcuts") {
                assert!(html.contains("data-slskr-native-select"));
            }
            assert!(html.contains("slskr-native-selection-status"));
            assert!(html.contains("slskr-toast-region"));
            assert!(
                html.contains(native_class),
                "route {path} should render native parity class {native_class}"
            );
            assert!(html.contains("data-slskr-parity-reference"));
            assert!(html.contains("data-react-component="));
            assert!(html.contains("slskr-route-summary"));
            assert!(html.contains("data-slskr-refresh-route"));

            let parity_index = html
                .find("data-slskr-parity-reference")
                .unwrap_or_else(|| panic!("missing protocol compatibility panel for route {path}"));
            let native_index = html
                .find("slskr-native-workspace")
                .unwrap_or_else(|| panic!("missing native workspace for route {path}"));
            assert!(
                parity_index < native_index,
                "route {path} should show compatibility content before native page body"
            );
        }
    }

    #[test]
    fn route_workflows_render_populated_api_rows() {
        let cases = [
            (
                "/searches/42",
                ApiEndpoint {
                    method: "GET",
                    path: "/searches/:id/responses",
                    surface: "search",
                },
                r#"[{"username":"peer-live","hasFreeUploadSlot":true,"queueLength":2,"files":[{"filename":"Artist/Album/01 Track.flac"}]}]"#,
                &[
                    "Artist/Album/01 Track.flac",
                    "peer-live",
                    "free slot / queue 2",
                ][..],
            ),
            (
                "/discovery-graph",
                ApiEndpoint {
                    method: "GET",
                    path: "/searches",
                    surface: "search",
                },
                r#"[{"id":42,"searchText":"public domain jazz","state":"Running"}]"#,
                &["public domain jazz", "search 42", "Running"][..],
            ),
            (
                "/playlist-intake",
                ApiEndpoint {
                    method: "POST",
                    path: "/source-feed-imports/preview",
                    surface: "source",
                },
                r#"[{"artist":"Archive Artist","title":"Public Domain Theme","status":"Matched"}]"#,
                &["Public Domain Theme", "Archive Artist", "Matched"][..],
            ),
            (
                "/wishlist",
                ApiEndpoint {
                    method: "GET",
                    path: "/wishlist",
                    surface: "wishlist",
                },
                r#"[{"searchText":"rare live set","filter":"flac","enabled":true,"autoDownload":false}]"#,
                &["rare live set", "flac", "enabled=true / auto=false"][..],
            ),
            (
                "/downloads",
                ApiEndpoint {
                    method: "GET",
                    path: "/transfers/downloads",
                    surface: "transfers",
                },
                r#"[{"username":"peer-down","files":[{"filename":"Remote/Song.mp3","state":"InProgress","progress":0.5,"speed":"1 MB/s"}]}]"#,
                &["Remote/Song.mp3", "peer-down", "InProgress / 50% / 1 MB/s"][..],
            ),
            (
                "/uploads",
                ApiEndpoint {
                    method: "GET",
                    path: "/transfers/uploads",
                    surface: "transfers",
                },
                r#"[{"username":"peer-up","files":[{"filename":"Local/Song.flac","state":"Queued","progress":0.25,"speed":"512 KB/s"}]}]"#,
                &["Local/Song.flac", "peer-up", "Queued / 25% / 512 KB/s"][..],
            ),
            (
                "/messages",
                ApiEndpoint {
                    method: "GET",
                    path: "/conversations",
                    surface: "messages",
                },
                r#"[{"username":"peer-msg","lastMessage":"hello","unreadCount":3}]"#,
                &["peer-msg", "hello", "3 unread"][..],
            ),
            (
                "/users",
                ApiEndpoint {
                    method: "GET",
                    path: "/users",
                    surface: "users",
                },
                r#"[{"username":"peer-user","status":"Online","sharedFileCount":100}]"#,
                &["peer-user", "Online", "100"][..],
            ),
            (
                "/contacts",
                ApiEndpoint {
                    method: "GET",
                    path: "/contacts",
                    surface: "contacts",
                },
                r#"[{"nickname":"Friend","peerId":"peer-contact","verified":true}]"#,
                &["Friend", "peer-contact", "verified=true"][..],
            ),
            (
                "/solid",
                ApiEndpoint {
                    method: "GET",
                    path: "/solid/status",
                    surface: "solid",
                },
                r#"{"webId":"https://example.test/profile#me","storage":"pod-a","status":"connected"}"#,
                &["https://example.test/profile#me", "pod-a", "connected"][..],
            ),
            (
                "/collections",
                ApiEndpoint {
                    method: "GET",
                    path: "/collections",
                    surface: "collections",
                },
                r#"[{"title":"Live Collection","type":"Playlist","itemCount":7}]"#,
                &["Live Collection", "Playlist", "7 items"][..],
            ),
            (
                "/sharegroups",
                ApiEndpoint {
                    method: "GET",
                    path: "/sharegroups",
                    surface: "sharegroups",
                },
                r#"[{"name":"Trusted peers","memberCount":2,"createdAt":"today"}]"#,
                &["Trusted peers", "2 members", "today"][..],
            ),
            (
                "/shared",
                ApiEndpoint {
                    method: "GET",
                    path: "/share-grants",
                    surface: "sharegroups",
                },
                r#"[{"id":"grant-1","title":"Shared Collection","owner":"peer-owner","permissions":"read"}]"#,
                &["Shared Collection", "peer-owner", "read"][..],
            ),
            (
                "/browse",
                ApiEndpoint {
                    method: "GET",
                    path: "/users/:username/browse",
                    surface: "browse",
                },
                r#"{"directories":[{"name":"Music","type":"folder","size":0}],"files":[{"filename":"Music/Track.flac","type":"file","size":12345}]}"#,
                &["Music", "Music/Track.flac", "Download"][..],
            ),
            (
                "/system",
                ApiEndpoint {
                    method: "GET",
                    path: "/server",
                    surface: "system",
                },
                r#"{"state":"Connected","username":"audit-user"}"#,
                &["Connection", "Connected", "audit-user"][..],
            ),
        ];

        for (route, endpoint, body, expected) in cases {
            let html = route_workspace_result_html(
                route,
                &[EndpointBody {
                    endpoint,
                    body: body.to_string(),
                }],
            );
            for value in expected {
                assert!(
                    html.contains(value),
                    "route {route} should render live workflow value {value}"
                );
            }
        }
    }

    #[test]
    fn array_data_cards_render_filterable_table_and_csv_views() {
        let response = EndpointBody {
            endpoint: ApiEndpoint {
                method: "GET",
                path: "/searches",
                surface: "search",
            },
            body: r#"[{"id":1,"query":"public domain jazz","status":"Completed","username":"peer1"},{"id":2,"query":"archive live set","status":"Running","username":"peer2"}]"#.to_string(),
        };
        let html = data_card_result_html(&response);
        assert!(html.contains("data-slskr-data-card"));
        assert!(html.contains("data-slskr-view=\"list\""));
        assert!(html.contains("slskr-card-filter"));
        assert!(html.contains("data-slskr-card-clear"));
        assert!(html.contains("data-slskr-card-count"));
        assert!(html.contains("2 / 2"));
        assert!(html.contains("data-slskr-card-view=\"table\""));
        assert!(html.contains("data-slskr-sort-index"));
        assert!(html.contains("slskr-data-table"));
        assert!(html.contains("2 records"));
        assert!(html.contains("public domain jazz"));
        assert!(html.contains("data-slskr-record-select"));
        assert!(html.contains("data-slskr-record-json"));
        assert!(html.contains("slskr-card-inspector"));
        assert!(html.contains("Record Inspector"));
        assert!(html.contains("CSV"));
    }

    #[test]
    fn shell_prioritizes_functional_webui_over_migration_inventory() {
        let html = shell_html();
        assert!(html.contains("slskr-appbar"));
        assert!(html.contains("Now Playing"));
        assert!(html.contains("Queue idle"));
        assert!(html.contains("slskr-page-data"));
        assert!(!html.contains("Rust web migration target"));
        assert!(!html.contains("Rust/WASM"));
        assert!(!html.contains("Bulk Endpoint Workbench"));
    }

    #[test]
    fn wishlist_history_renderer_escapes_remote_fields_and_rejects_malformed_json() {
        let html = wishlist_history_response_html(
            r#"[{"searchText":"<script>alert(1)</script>","status":"completed<img>","startedAt":"now&later","result_count":2}]"#,
        );
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("completed&lt;img&gt;"));
        assert!(html.contains("now&amp;later"));
        assert!(!html.contains("<script>"));
        assert_eq!(
            wishlist_history_response_html("not json"),
            "<p>Run history could not be read.</p>"
        );
    }

    #[test]
    fn transfer_attempt_renderer_escapes_attempt_history() {
        let html = transfer_attempts_response_html(
            r#"{"attempts":[{"peer_username":"peer<script>","status":"failed<img>","filename":"Remote/A&B.flac"}]}"#,
        );
        assert!(html.contains("peer&lt;script&gt;"));
        assert!(html.contains("failed&lt;img&gt;"));
        assert!(html.contains("Remote/A&amp;B.flac"));
        assert!(!html.contains("<script>"));
        assert_eq!(
            transfer_attempts_response_html("bad"),
            "<p>Attempt history could not be read.</p>"
        );
    }

    #[test]
    fn rust_ui_parity_ledger_tracks_closure_instead_of_stale_gaps() {
        let ledger = include_str!("../../../docs/rust-ui-parity-ledger.md");
        assert!(ledger.contains("Estimated completion: 95-98%."));
        assert!(ledger.contains("## Route Closure"));
        assert!(ledger.contains("live-backend behavioral validation"));
        assert!(!ledger.contains("Estimated completion: 55-65%."));
        assert!(!ledger.contains("Remaining 1:1 Gaps"));
        assert!(!ledger.contains("| Route | Current Rust Coverage | Remaining 1:1 Gaps |"));
    }

    #[test]
    fn route_probe_urls_use_concrete_paths() {
        let endpoint = ApiEndpoint {
            method: "GET",
            path: "/searches/:id/responses",
            surface: "search",
        };
        assert_eq!(
            concrete_endpoint_path("/searches/42", endpoint),
            "/api/v0/searches/42/responses"
        );
        assert_eq!(
            concrete_endpoint_path("/searches/<script>", endpoint),
            "/api/v0/searches/1/responses"
        );
        let pending = route_probe_pending_html("/messages");
        assert!(pending.contains("/api/v0/conversations"));
        assert!(pending.contains("/api/v0/conversations/peer1"));
    }

    #[test]
    fn rust_actions_render_core_mutations() {
        let html = route_page_html("/searches/42");
        assert!(html.contains("Start Search"));
        assert!(html.contains("Stop Search"));
        assert!(html.contains("Remove Search"));
        assert!(html.contains("Clear Searches"));
        assert!(html.contains("/api/v0/searches/42"));
        assert!(html.contains("data-slskr-action-body=\"SearchText\""));

        let transfers = route_page_html("/downloads");
        assert!(transfers.contains("Queue Download"));
        assert!(transfers.contains("Enable Accelerated Downloads"));
        assert!(transfers.contains("Disable Accelerated Downloads"));
        assert!(transfers.contains("/api/v0/transfers/downloads/peer1"));

        let rooms = route_page_html("/rooms");
        assert!(rooms.contains("Join Room"));
        assert!(rooms.contains("Send Room Message"));
        assert!(rooms.contains("Leave Room"));
        assert!(rooms.contains("/api/v0/rooms/joined/contract-room/messages"));

        let messages = route_page_html("/messages");
        assert!(messages.contains("Send Message"));
        assert!(messages.contains("Acknowledge Conversation"));
        assert!(messages.contains("Delete Conversation"));

        let system = route_page_html("/system/network");
        assert!(system.contains("Connect"));
        assert!(system.contains("Disconnect"));
        assert!(system.contains("Rescan Shares"));
        assert!(system.contains("/api/v0/server"));

        let wishlist = route_page_html("/wishlist");
        assert!(wishlist.contains("Add Wishlist Item"));
        assert!(wishlist.contains("Run Wishlist Search"));

        let contacts = route_page_html("/contacts");
        assert!(contacts.contains("Add Contact"));
        assert!(contacts.contains("Watch User"));
        assert!(contacts.contains("Add User Note"));

        let collections = route_page_html("/collections");
        assert!(collections.contains("Create Collection"));
        assert!(collections.contains("Create Share Group"));
        assert!(collections.contains("Create Share Grant"));
        assert!(collections.contains("Backfill Share Grant"));
        assert!(collections.contains("Add Item to Collection"));

        let integrations = route_page_html("/playlist-intake");
        assert!(integrations.contains("Preview Playlist"));
        assert!(integrations.contains("Build Discovery Graph"));
        assert!(integrations.contains("Track MusicBrainz Target"));
        assert!(integrations.contains("Create SongID Run"));
    }

    #[test]
    fn native_workflow_labels_resolve_to_real_route_actions() {
        let expectations = [
            ("/searches", "Search", "Start Search"),
            ("/discovery-graph", "Build Atlas", "Build Discovery Graph"),
            ("/playlist-intake", "Import Playlist", "Preview Playlist"),
            ("/wishlist", "Run Enabled", "Run Wishlist Search"),
            ("/downloads", "Clear Completed", "Clear Completed Downloads"),
            ("/downloads", "Cancel All", "Cancel Download"),
            ("/uploads", "Clear Completed", "Clear Completed Uploads"),
            ("/uploads", "Allow selected", "Allow Upload"),
            ("/uploads", "Deny selected", "Deny Upload"),
            ("/messages", "Reply", "Send Message"),
            ("/messages", "Delete Conversation", "Delete Conversation"),
            ("/users", "Watch", "Watch User"),
            ("/users", "Message", "Send Message"),
            ("/contacts", "Add Friend", "Add Contact"),
            ("/contacts", "Message", "Send Message"),
            ("/contacts", "Browse", "Request Directory"),
            ("/contacts", "Remove", "Remove Contact"),
            ("/contacts", "Create Invite", "Create Invite"),
            ("/contacts", "Refresh Nearby", "Refresh Nearby"),
            ("/solid", "Resolve WebID", "Resolve WebID"),
            ("/collections", "Open", "Open Collection"),
            ("/collections", "Add Item", "Add Item to Collection"),
            ("/collections", "Share", "Create Share Grant"),
            ("/sharegroups", "Create Share Grant", "Create Share Grant"),
            ("/sharegroups", "Update Share Grant", "Update Share Grant"),
            ("/sharegroups", "Issue Token", "Issue Share Token"),
            ("/shared", "Open", "Open Shared Manifest"),
            ("/shared", "Backfill", "Backfill Share Grant"),
            ("/browse", "Download Selected", "Queue Download"),
            ("/system", "Check for Updates", "Check for Updates"),
            ("/system", "Get Privileges", "Get Privileges"),
            ("/system", "Diagnostic Bundle", "Diagnostic Bundle"),
            ("/system", "Setup Health", "Setup Health"),
            ("/system", "Vacuum database", "Vacuum Database"),
            ("/system", "Check Lidarr", "Check Lidarr"),
            ("/system", "Refresh Lidarr Sync", "Refresh Lidarr Sync"),
            (
                "/system",
                "Track MusicBrainz Target",
                "Track MusicBrainz Target",
            ),
            ("/system", "Refresh MusicBrainz", "Refresh MusicBrainz"),
            ("/system", "Start SongID Run", "Start SongID Run"),
            ("/system", "Refresh SongID", "Refresh SongID"),
            (
                "/system",
                "Scan Library Health",
                "Start Library Health Scan",
            ),
            ("/system", "Fix Library Issues", "Fix Library Issues"),
        ];

        for (path, label, expected_action) in expectations {
            let action = route_action_for_native_label(path, label)
                .unwrap_or_else(|| panic!("{path} {label} should resolve"));
            assert_eq!(action.label, expected_action);
        }
    }

    #[test]
    fn native_action_fallbacks_are_domain_specific() {
        assert!(native_action_fallback(ActionBody::SearchText).is_empty());
        assert!(native_action_fallback(ActionBody::FeedPreview).is_empty());
        assert!(native_action_fallback(ActionBody::DownloadFiles).is_empty());
        assert_eq!(native_action_fallback(ActionBody::BrowseDirectory), "/");
        assert!(native_action_fallback(ActionBody::Username).is_empty());
        assert!(native_action_fallback(ActionBody::None).is_empty());
    }

    #[test]
    fn native_subpanels_cover_deep_route_workflows() {
        let system = route_page_html("/system");
        for value in [
            "MediaCore",
            "Security Policies",
            "Library Health",
            "Quarantine Jury",
            "slskr-system-panel-table",
            "Outbound webhooks",
            "MusicBrainz",
            "Raw metrics",
            "Source providers",
            "Scan Library Health",
            "Vacuum Database",
            "Proxy trust",
        ] {
            assert!(
                system.contains(value),
                "system panel should contain {value}"
            );
        }

        let browse = route_page_html("/browse");
        for value in [
            "Open a New Browse Tab",
            "Breadcrumb",
            "Multi-select",
            "Download Selected",
            "data-slskr-browse-workspace",
            "data-slskr-browse-session",
            "data-slskr-browse-folder",
            "data-slskr-browse-download-manifest",
            "File filter",
            "Refresh Folder",
            "Estimated queue impact appears after selection",
        ] {
            assert!(
                browse.contains(value),
                "browse panel should contain {value}"
            );
        }

        let messages = route_page_html("/messages");
        for value in [
            "Delete Conversation",
            "Unread",
            "Pods",
            "Compose",
            "data-slskr-messages-workspace",
            "data-slskr-message-lifecycle",
            "data-slskr-room-state",
            "data-slskr-pod-state",
            "data-slskr-thread-state",
            "data-slskr-message-transcript",
            "data-slskr-message-actions",
            "data-slskr-compose-history",
            "Search conversations",
            "Clear Search",
            "data-slskr-message-gate-panel",
            "Gate reply",
            "data-slskr-message-gate-save",
            "cooldownMinutes",
        ] {
            assert!(
                messages.contains(value),
                "messages panel should contain {value}"
            );
        }
        let live_messages = route_workspace_result_html(
            "/messages",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/private-message-auto-response",
                    surface: "messages",
                },
                body: r#"{"enabled":true,"message_configured":true,"cooldown_minutes":15,"runtimeMutable":true}"#.to_string(),
            }],
        );
        assert!(live_messages.contains("<mark>armed</mark>"));
        assert!(live_messages.contains(r#"value="15""#));
        assert!(!live_messages.contains("Runtime human check"));

        let sharing = populated_route_html("/sharegroups");
        for value in ["Create Share Grant", "Update Share Grant", "Permissions"] {
            assert!(
                sharing.contains(value),
                "share groups panel should contain {value}"
            );
        }
    }

    #[test]
    fn native_primary_workspaces_include_react_like_structures() {
        for path in [
            "/searches",
            "/discovery-graph",
            "/playlist-intake",
            "/wishlist",
            "/downloads",
            "/uploads",
            "/messages",
            "/users",
            "/contacts",
            "/solid",
            "/collections",
            "/sharegroups",
            "/shared",
            "/browse",
            "/system",
        ] {
            let page = route_page_html(path);
            assert!(
                page.contains("data-slskr-native-preview-title"),
                "{path} should expose a row-driven native preview title"
            );
            assert!(
                page.contains("data-slskr-native-preview-count"),
                "{path} should expose a row-driven native preview count"
            );
            assert!(
                page.contains("data-slskr-native-preview-fields"),
                "{path} should expose row-driven native preview fields"
            );
            assert!(
                page.contains("data-slskr-native-inspector-fields"),
                "{path} should expose row-driven native inspector fields"
            );
            assert!(
                page.contains("data-slskr-native-inspector-actions"),
                "{path} should expose selected-row context actions in the inspector"
            );
        }

        let search = route_page_html("/searches");
        for value in [
            "Search Detail",
            "No search selected",
            "Duplicate folding",
            "data-slskr-search-filter-modal",
            "Fold duplicate results",
            "Search ranking profile",
            "Hide locked files",
            "data-slskr-search-expansion",
        ] {
            assert!(
                search.contains(value),
                "search workspace should contain {value}"
            );
        }

        let discovery = route_page_html("/discovery-graph");
        for value in [
            "Discovery Graph Atlas",
            "No graph node selected",
            "Queue Nearby",
        ] {
            assert!(
                discovery.contains(value),
                "discovery workspace should contain {value}"
            );
        }

        let playlist = route_page_html("/playlist-intake");
        for value in [
            "Import validation",
            "No playlist row selected",
            "Import Playlist",
        ] {
            assert!(
                playlist.contains(value),
                "playlist workspace should contain {value}"
            );
        }

        let wishlist = route_page_html("/wishlist");
        for value in [
            "Request Portal Summary",
            "No wishlist item selected",
            "Run Enabled",
        ] {
            assert!(
                wishlist.contains(value),
                "wishlist workspace should contain {value}"
            );
        }

        let downloads = route_page_html("/downloads");
        for value in ["Transfer Group", "No downloads selected", "Retry All"] {
            assert!(
                downloads.contains(value),
                "downloads workspace should contain {value}"
            );
        }
        assert!(
            downloads.contains("data-slskr-transfer-state-control")
                || downloads.contains("No downloads to display")
        );

        let uploads = route_page_html("/uploads");
        for value in ["Transfer Group", "No uploads selected"] {
            assert!(
                uploads.contains(value),
                "uploads workspace should contain {value}"
            );
        }

        let users = route_page_html("/users");
        for value in ["User Detail", "No user selected", "Save note"] {
            assert!(
                users.contains(value),
                "users workspace should contain {value}"
            );
        }

        let contacts = route_page_html("/contacts");
        for value in ["All Contacts", "No contact selected", "Refresh Nearby"] {
            assert!(
                contacts.contains(value),
                "contacts workspace should contain {value}"
            );
        }

        let solid = route_page_html("/solid");
        for value in [
            "Identity Document",
            "No Solid resource selected",
            "Resolve WebID",
            "Sync Storage",
        ] {
            assert!(
                solid.contains(value),
                "solid workspace should contain {value}"
            );
        }

        let system = route_page_html("/system");
        for value in [
            "Operator Actions",
            "No system item selected",
            "Diagnostic Bundle",
        ] {
            assert!(
                system.contains(value),
                "system workspace should contain {value}"
            );
        }

        let messages = route_page_html("/messages");
        for value in [
            "Thread Workspace",
            "slskr-native-thread-grid",
            "data-slskr-native-preview-title",
            "data-slskr-native-preview-count",
            "No conversation selected",
            "Delete Conversation",
            "No pod channels returned",
        ] {
            assert!(
                messages.contains(value),
                "messages workspace should contain {value}"
            );
        }

        let browse = route_page_html("/browse");
        for value in [
            "Directory Tree",
            "slskr-native-breadcrumb",
            "Download Preview",
            "data-slskr-native-preview-title",
            "Preserve folders",
            "Duplicate warning review",
        ] {
            assert!(
                browse.contains(value),
                "browse workspace should contain {value}"
            );
        }

        let collections = route_page_html("/collections");
        for value in [
            "Item Picker",
            "data-slskr-native-preview-title",
            "Already in collection warning",
            "Audience picker",
            "Stream/download policies",
        ] {
            assert!(
                collections.contains(value),
                "collections workspace should contain {value}"
            );
        }

        let sharegroups = route_page_html("/sharegroups");
        for value in [
            "Grant Matrix",
            "data-slskr-native-preview-title",
            "Token revoke",
            "Grant audit trail",
            "Create Share Grant",
        ] {
            assert!(
                sharegroups.contains(value),
                "share groups workspace should contain {value}"
            );
        }

        let shared = populated_route_html("/shared");
        for value in [
            "Shared Manifest",
            "file-level access preview",
            "data-slskr-native-preview-title",
            "Backfill selected collection",
        ] {
            assert!(
                shared.contains(value),
                "shared workspace should contain {value}"
            );
        }
    }

    #[test]
    fn native_tables_expose_domain_row_action_sets() {
        let expectations = [
            ("/searches", &["Preview", "Download"][..]),
            ("/discovery-graph", &["Queue Nearby", "Build Atlas"]),
            ("/playlist-intake", &["Import Playlist", "Queue Plans"]),
            ("/wishlist", &["Run Enabled", "Copy Review"]),
            ("/downloads", &["Retry", "Cancel", "Remove"]),
            ("/uploads", &["Allow selected", "Deny selected"]),
            (
                "/messages",
                &["Reply", "Acknowledge", "Delete Conversation"],
            ),
            ("/users", &["Message", "Watch", "Save note"]),
            ("/contacts", &["Message", "Browse", "Remove"]),
            (
                "/solid",
                &["Resolve WebID", "Connect Identity", "Sync Storage"],
            ),
            ("/collections", &["Add Item", "Share", "Remove item"]),
            (
                "/sharegroups",
                &[
                    "Add Member",
                    "Issue Token",
                    "Create Share Grant",
                    "Update Share Grant",
                ],
            ),
            ("/shared", &["Stream", "Backfill", "Copy token"]),
            ("/browse", &["Download Selected", "Open a New Browse Tab"]),
            (
                "/system",
                &["Rescan shares", "Vacuum database", "Diagnostic Bundle"],
            ),
        ];

        for (path, labels) in expectations {
            let html = populated_route_html(path);
            assert!(
                html.contains("slskr-native-row-actions"),
                "{path} should render row action toolbar"
            );
            assert!(
                html.contains("data-slskr-native-action-menu"),
                "{path} should expose selected-row action menu data"
            );
            for label in labels {
                assert!(
                    html.contains(label),
                    "{path} row action toolbar should contain {label}"
                );
            }
        }
    }

    #[test]
    fn native_editor_surfaces_cover_modal_workflows() {
        let expectations = [
            (
                "/wishlist",
                &[
                    "data-slskr-native-editor",
                    "Wishlist Editor",
                    "Auto-download",
                    "Discovery Inbox bridge",
                ][..],
            ),
            (
                "/users",
                &[
                    "data-slskr-native-editor",
                    "User Note Editor",
                    "Privileges and stats",
                    "Save note",
                ],
            ),
            (
                "/contacts",
                &[
                    "data-slskr-native-editor",
                    "Contact Editor",
                    "Create Invite",
                    "Groups and notes",
                ],
            ),
            (
                "/collections",
                &[
                    "data-slskr-native-editor",
                    "Collection Editor",
                    "Audience",
                    "Remove item",
                ],
            ),
            (
                "/sharegroups",
                &[
                    "data-slskr-native-editor",
                    "Share Grant Editor",
                    "Permissions",
                    "Issue Token",
                ],
            ),
            (
                "/shared",
                &[
                    "data-slskr-native-editor",
                    "Inbound Access Editor",
                    "Copy token",
                ],
            ),
            (
                "/system",
                &[
                    "data-slskr-native-editor",
                    "Settings Editor",
                    "Option key",
                    "Diagnostic Bundle",
                ],
            ),
        ];

        for (path, labels) in expectations {
            let html = route_page_html(path);
            for label in labels {
                assert!(
                    html.contains(label),
                    "{path} native editor should contain {label}"
                );
            }
        }

        let search = route_page_html("/searches");
        assert!(
            !search.contains("data-slskr-native-editor"),
            "search should keep its planner in the route workspace instead of the editor modal surface"
        );
    }

    #[test]
    fn native_shell_contains_in_app_confirmation_modal_styles() {
        let html = route_page_html("/downloads");
        assert!(html.contains("Cancel"));
        assert!(html.contains("Remove"));
        let css = include_str!("../static/styles.css");
        for value in [
            "slskr-modal-backdrop",
            "slskr-modal",
            "data-slskr-confirm-run",
        ] {
            assert!(
                css.contains(value),
                "confirmation modal CSS should contain {value}"
            );
        }
    }

    #[test]
    fn rust_action_bodies_are_json_safe() {
        assert_eq!(
            action_body_from_value(ActionBody::SearchText, "a \"b\"").unwrap(),
            r#"{"searchText":"a \"b\""}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::MusicBrainzTarget, "release-1").unwrap(),
            r#"{"releaseId":"release-1"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::SongIdSource, "query").unwrap(),
            r#"{"source":"query"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::BrowseDirectory, "Music\\Jazz\nLive").unwrap(),
            r#"{"directory":"Music\\Jazz\nLive"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::JsonString, "room\t<script>").unwrap(),
            "\"room\\t<script>\""
        );
        assert_eq!(
            action_body_from_value(ActionBody::DownloadFiles, "Remote/Track.flac").unwrap(),
            r#"[{"filename":"Remote/Track.flac","size":99}]"#
        );
        assert_eq!(
            action_body_from_value(
                ActionBody::DownloadFiles,
                "Remote/Track.flac\nRemote/Other.mp3"
            )
            .unwrap(),
            r#"[{"filename":"Remote/Track.flac","size":99},{"filename":"Remote/Other.mp3","size":99}]"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::EnabledTrue, "ignored").unwrap(),
            r#"{"enabled":true}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::EnabledFalse, "ignored").unwrap(),
            r#"{"enabled":false}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::Username, "peer1").unwrap(),
            r#"{"username":"peer1","note":"Created from the Rust web UI"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::Permissions, "").unwrap(),
            r#"{"permissions":"read"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::ShareGrant, "peer1").unwrap(),
            r#"{"collection_id":"","username":"peer1"}"#
        );
        assert_eq!(
            action_body_from_value(ActionBody::ShareGroupMember, "peer1").unwrap(),
            r#"{"userId":"peer1"}"#
        );
        assert!(
            action_body_from_value(ActionBody::FeedPreview, "artist - song")
                .unwrap()
                .contains("\"sourceText\":\"artist - song\"")
        );
        assert!(action_body_from_value(ActionBody::None, "ignored").is_none());
    }

    #[test]
    fn native_row_actions_are_marked_for_selected_row_execution() {
        let html = populated_route_html("/browse");
        assert!(html.contains(r#"data-slskr-native-row-action="Download Selected""#));
        assert!(html.contains(r#"data-slskr-native-row-action="Open a New Browse Tab""#));
        assert!(html.contains(r#"data-slskr-native-title="/Music/Open Sessions""#));
        assert!(html.contains(r#"data-slskr-native-resource-id="row-1""#));
        let users = populated_route_html("/users");
        assert!(users.contains(r#"data-slskr-native-resource-id="peer1""#));
    }

    #[test]
    fn native_rows_expose_structured_domain_action_values() {
        let search = route_workspace_result_html(
            "/searches",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/searches/:id/responses",
                    surface: "search",
                },
                body: r#"[{"username":"peer1","files":[{"filename":"Archive/Track.flac"}],"queueLength":2,"hasFreeUploadSlot":true}]"#.to_string(),
            }],
        );
        assert!(search.contains(r#"data-slskr-native-filename="Archive/Track.flac""#));
        assert!(search.contains(r#"data-slskr-native-peer="peer1""#));
        assert!(search.contains(r#"data-slskr-native-queue-state="free slot / queue 2""#));
        assert!(search.contains("data-slskr-search-result-controls"));
        assert!(search.contains("Expand Result"));
        assert!(search.contains("Fold Duplicates"));
        assert!(search.contains("Smart rank"));

        let downloads = route_workspace_result_html(
            "/downloads",
            &[
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/transfers/downloads",
                        surface: "transfers",
                    },
                    body: r#"[{"id":77,"username":"peer2","filename":"Remote/Song.mp3","state":"Queued","progress":0.5}]"#.to_string(),
                },
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/downloads/requests",
                        surface: "transfers",
                    },
                    body: r#"[{"request":{"id":"11111111-1111-4111-8111-111111111111","name":"Archive Cut","originalFilename":"Remote/Song.mp3","size":1200,"state":"Failed","bitRate":320,"sampleRate":48000,"bitDepth":24,"length":180,"artist":"Archive Artist","title":"Song"},"attemptCount":2,"current":{"id":77,"peer_username":"peer2","bytes_transferred":600,"recovery_action":"retry","recovery_label":"Find other sources"}}]"#.to_string(),
                },
            ],
        );
        assert!(downloads.contains(r#"data-slskr-native-filename="Remote/Song.mp3""#));
        assert!(downloads.contains(r#"data-slskr-native-peer="peer2""#));
        assert!(downloads
            .contains(r#"data-slskr-native-transfer-state="Queued / 50% / 0 B/s / id=77""#));
        assert!(downloads.contains(r#"data-slskr-native-transfer-id="77""#));
        assert!(downloads.contains(
            r#"data-slskr-native-action-summary="Cancel download 77 from peer2: Remote/Song.mp3""#
        ));
        assert!(downloads.contains(
            r#"data-slskr-native-detail-list="File: Remote/Song.mp3 | Peer: peer2 | Download state: Queued / 50% / 0 B/s / id=77 | Next action: Cancel""#
        ));
        assert!(downloads.contains(r#"data-slskr-native-action-menu="Cancel | Retry | Remove""#));
        assert!(downloads.contains(r#"<meter min="0" max="100" value="50""#));
        assert!(downloads.contains("data-slskr-transfer-state-control"));
        for value in [
            "data-slskr-transfer-request-workspace",
            "Downloads and attempts",
            "Archive Cut",
            "Archive Artist — Song",
            "Find other sources",
            "data-slskr-transfer-column-toggle=\"bitrate\"",
            "data-slskr-transfer-retry=\"77\"",
            "data-slskr-transfer-attempts",
            "data-slskr-transfer-rename-save",
        ] {
            assert!(
                downloads.contains(value),
                "downloads should contain {value}"
            );
        }

        let contacts = route_workspace_result_html(
            "/contacts",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/contacts",
                    surface: "identity",
                },
                body: r#"[{"nickname":"Nick","peerId":"peer3","group":"trusted","verified":true}]"#
                    .to_string(),
            }],
        );
        assert!(contacts.contains(r#"data-slskr-native-contact="Nick""#));
        assert!(contacts.contains(r#"data-slskr-native-username="peer3""#));

        let wishlist = populated_route_html("/wishlist");
        assert!(wishlist.contains("data-slskr-native-search-filter"));
        assert!(wishlist.contains("data-slskr-wishlist-ignore-manager"));
        assert!(wishlist.contains("Ignored result folders"));
        assert_eq!(
            wishlist.matches("data-slskr-native-filter ").count(),
            1,
            "only the filter input should use data-slskr-native-filter"
        );

        let live_wishlist = route_workspace_result_html(
            "/wishlist",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/wishlist",
                    surface: "wishlist",
                },
                body: r#"[{"id":"wish-7","searchText":"rare live set","filter":"flac","enabled":true,"autoDownload":true,"maxResults":25,"maxDownloads":3,"lastSearchedAt":200,"lastViewedAt":100,"lastVisibleHitCount":8,"lastHiddenLockedHitCount":2,"lastFilteredOutHitCount":4,"lastIgnoredResultHitCount":1,"lastResponseCount":6,"totalSearchCount":9,"totalDownloadCount":2,"ignoredResultCount":1,"ignoredResults":[{"id":"rule-9","username":"noisy-peer","directory":"Bootlegs/Unsorted"}]}]"#
                    .to_string(),
            }],
        );
        assert!(live_wishlist.contains(r#"data-slskr-native-wishlist-id="wish-7""#));
        assert!(live_wishlist
            .contains(r#"data-slskr-native-action-summary="Run wishlist wish-7: rare live set""#));
        assert!(live_wishlist.contains(
            r#"data-slskr-native-detail-list="Wanted search: rare live set | Filter: flac | Automation: enabled=true / auto=true / id=wish-7 | Next action: Run""#
        ));
        assert!(live_wishlist
            .contains(r#"data-slskr-native-action-menu="Run | Run Enabled | Copy Review""#));
        assert!(live_wishlist.contains("data-slskr-wishlist-row-controls"));
        assert!(live_wishlist.contains(r#"aria-label="Enabled rare live set" checked"#));
        assert!(live_wishlist.contains("data-slskr-wishlist-policy-manager"));
        assert!(live_wishlist.contains("Search policy and history"));
        assert!(live_wishlist.contains("new results"));
        assert!(live_wishlist.contains("<strong>8</strong> visible"));
        assert!(live_wishlist.contains(r#"data-slskr-wishlist-policy-save"#));
        assert!(live_wishlist.contains(r#"data-slskr-wishlist-mark-viewed"#));
        assert!(live_wishlist.contains(r#"data-slskr-wishlist-load-history"#));
        assert!(live_wishlist.contains(r#"data-slskr-wishlist-policy-field="maxDownloads""#));
        assert!(live_wishlist.contains("Noise gate"));
        assert!(live_wishlist.contains("noisy-peer"));
        assert!(live_wishlist.contains("Bootlegs/Unsorted"));
        assert!(live_wishlist.contains(r#"data-slskr-wishlist-ignore-rule-id="rule-9""#));
        assert!(live_wishlist.contains(r#"data-slskr-wishlist-ignore-submit"#));
        assert!(live_wishlist.contains(">Restore</button>"));

        let history = wishlist_history_response_html(
            r#"[{"searchText":"rare live set","status":"completed","startedAt":"200","result_count":8}]"#,
        );
        assert!(history.contains("slskr-wishlist-history-list"));
        assert!(history.contains("rare live set"));
        assert!(history.contains("completed · 8 results"));

        let sharing = populated_route_html("/sharegroups");
        assert!(sharing.contains("data-slskr-native-share-group"));
        assert!(sharing.contains("data-slskr-native-member-count"));
        assert!(sharing.contains("data-slskr-share-group-row-controls"));

        let shared = populated_route_html("/shared");
        assert!(shared.contains("data-slskr-native-owner"));
        assert!(shared.contains("data-slskr-native-permissions"));
        assert!(shared.contains("data-slskr-inbound-permission-controls"));

        let browse = populated_route_html("/browse");
        assert!(browse.contains("data-slskr-native-path"));
        assert!(browse.contains("data-slskr-native-entry-kind"));
        assert!(browse.contains("data-slskr-native-filename"));
        assert!(browse.contains("data-slskr-browse-entry-controls"));
    }

    #[test]
    fn rust_action_paths_reject_untrusted_route_params() {
        let endpoint = RouteAction {
            body: ActionBody::None,
            label: "Cancel Search",
            method: "DELETE",
            path: "/searches/:id",
            surface: "search",
        };
        assert_eq!(
            concrete_action_path("/searches/42", endpoint),
            "/api/v0/searches/42"
        );
        assert_eq!(
            concrete_action_path("/searches/<script>", endpoint),
            "/api/v0/searches/1"
        );
        assert_eq!(
            concrete_action_path("/searches", endpoint),
            "/api/v0/searches/1"
        );
        let transfer = RouteAction {
            body: ActionBody::None,
            label: "Cancel Download",
            method: "DELETE",
            path: "/transfers/downloads/:username/:id",
            surface: "transfers",
        };
        assert_eq!(
            concrete_action_path_with_target_and_id(
                "/downloads",
                transfer,
                Some("peer2"),
                Some("77")
            ),
            "/api/v0/transfers/downloads/peer2/77"
        );
        let wishlist = RouteAction {
            body: ActionBody::None,
            label: "Run Wishlist Search",
            method: "POST",
            path: "/wishlist/:id/search",
            surface: "wishlist",
        };
        assert_eq!(
            concrete_action_path_with_target_and_id(
                "/wishlist",
                wishlist,
                Some("ignored-peer"),
                Some("wish-7")
            ),
            "/api/v0/wishlist/wish-7/search"
        );
        let html = route_actions_html("/searches/<script>");
        assert!(html.contains("/api/v0/searches/1"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn route_action_lookup_uses_current_route_surface() {
        let search = route_action_at("/searches/42", 0).unwrap();
        assert_eq!(search.label, "Start Search");
        assert_eq!(
            concrete_action_path("/searches/42", search),
            "/api/v0/searches"
        );

        let remove = route_action_at("/searches/42", 2).unwrap();
        assert_eq!(remove.label, "Remove Search");
        assert_eq!(
            concrete_action_path("/searches/42", remove),
            "/api/v0/searches/42"
        );

        let browse = route_action_for_native_label("/browse", "Open a New Browse Tab")
            .expect("browse action");
        assert_eq!(
            concrete_action_path_with_target("/browse", browse, Some("browse-peer")),
            "/api/v0/users/browse-peer/directory"
        );
        assert_eq!(
            concrete_action_path_with_target("/browse", browse, Some("../bad")),
            "/api/v0/users/peer1/directory"
        );
        let remove_item = route_action_at("/searches", 2).expect("remove search");
        assert_eq!(
            concrete_action_path_with_target("/searches", remove_item, Some("search-42")),
            "/api/v0/searches/search-42"
        );

        let collection_item =
            route_action_for_native_label("/collections", "Remove Collection Item")
                .expect("collection item action");
        assert_eq!(
            concrete_action_path_with_target_and_id(
                "/collections",
                collection_item,
                Some("collection-7"),
                Some("item-9"),
            ),
            "/api/v0/collections/collection-7/items/item-9"
        );
        assert_eq!(
            concrete_action_path_with_target_and_id(
                "/collections",
                collection_item,
                Some("../bad"),
                Some("item-9"),
            ),
            "/api/v0/collections/1/items/item-9"
        );

        assert!(route_action_at("/searches/42", usize::MAX).is_none());
        assert!(route_action_at("/not-a-route", 0).is_none());
    }

    #[test]
    fn rust_route_summaries_parse_live_response_shapes() {
        let search = route_summary_result_html(
            "/searches/42",
            &[
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/searches/records",
                        surface: "search",
                    },
                    body: r#"{"entries":[{"id":"1"},{"id":"2"}]}"#.to_string(),
                },
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/searches/:id/responses",
                        surface: "search",
                    },
                    body: r#"[{"username":"peer1"}]"#.to_string(),
                },
            ],
        );
        assert!(search.contains(">2<"));
        assert!(search.contains(">1<"));
        assert!(search.contains("active records"));

        let transfers = route_summary_result_html(
            "/downloads",
            &[
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/transfers/downloads",
                        surface: "transfers",
                    },
                    body: r#"[{"username":"peer1"}]"#.to_string(),
                },
                EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/transfers/uploads",
                        surface: "transfers",
                    },
                    body: "[]".to_string(),
                },
            ],
        );
        assert!(transfers.contains("Downloads"));
        assert!(transfers.contains(">1<"));
        assert!(transfers.contains("Uploads"));

        let rooms = route_summary_result_html(
            "/rooms",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/rooms/joined",
                    surface: "rooms",
                },
                body: r#"["contract-room"]"#.to_string(),
            }],
        );
        assert!(rooms.contains("Joined"));
        assert!(rooms.contains(">1<"));
    }

    #[test]
    fn native_workspaces_parse_nested_live_domain_payloads() {
        let downloads = route_workspace_result_html(
            "/downloads",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/transfers/downloads",
                    surface: "transfers",
                },
                body: r#"[{"username":"parent-peer","state":"InProgress","bytesPerSecond":2048,"eta":"00:42","files":[{"filename":"Album/Track.flac","progress":0.625}]}]"#
                    .to_string(),
            }],
        );
        assert!(downloads.contains("Album/Track.flac"));
        assert!(downloads.contains("parent-peer"));
        assert!(downloads.contains("InProgress / 62% / 2048 B/s / ETA 00:42"));

        let browse = route_workspace_result_html(
            "/browse",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/users/:username/browse",
                    surface: "browse",
                },
                body: r#"{"directories":[{"name":"Music","isDirectory":true,"files":[{"filename":"Music/live.mp3","size":321}]}],"root":{"files":[{"name":"root.flac","fileSize":654}]}}"#
                    .to_string(),
            }],
        );
        assert!(browse.contains("Music"));
        assert!(browse.contains("folder"));
        assert!(browse.contains("Music/live.mp3"));
        assert!(browse.contains("root.flac"));

        let collections = route_workspace_result_html(
            "/collections",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/collections",
                    surface: "collections",
                },
                body:
                    r#"[{"title":"Road Trips","kind":"playlist","items":[{"id":"1"},{"id":"2"}]}]"#
                        .to_string(),
            }],
        );
        assert!(collections.contains("Road Trips"));
        assert!(collections.contains("2 items"));

        let sharegroups = route_workspace_result_html(
            "/sharegroups",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/sharegroups",
                    surface: "collections",
                },
                body: r#"[{"name":"Trusted","members":[{"username":"peer1"},{"username":"peer2"}],"createdAt":"today"}]"#
                    .to_string(),
            }],
        );
        assert!(sharegroups.contains("Trusted"));
        assert!(sharegroups.contains("2 members"));

        let shared = route_workspace_result_html(
                "/shared",
                &[EndpointBody {
                    endpoint: ApiEndpoint {
                        method: "GET",
                        path: "/share-grants",
                        surface: "collections",
                    },
                    body: r#"[{"id":"grant-nested","collection":{"title":"Inbox"},"owner":{"username":"sender"},"grant":{"permissions":"read/write"}}]"#
                        .to_string(),
                }],
        );
        assert!(shared.contains("Inbox"));
        assert!(shared.contains("sender"));
        assert!(shared.contains("read/write"));
    }

    #[test]
    fn rust_route_summaries_escape_live_response_values() {
        let html = route_summary_result_html(
            "/messages",
            &[EndpointBody {
                endpoint: ApiEndpoint {
                    method: "GET",
                    path: "/conversations/:username",
                    surface: "messages",
                },
                body: r#"{"username":"<script>"}"#.to_string(),
            }],
        );
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn bulk_surface_matrix_covers_every_route_group() {
        let matrix = surface_matrix_html();
        for surface in [
            "browse",
            "collections",
            "identity",
            "integrations",
            "messages",
            "rooms",
            "search",
            "system",
            "transfers",
            "wishlist",
        ] {
            assert!(matrix.contains(surface), "missing surface {surface}");
            assert!(surface_route_count(surface) > 0, "no routes for {surface}");
            assert!(
                !route_endpoints(surface).is_empty(),
                "no endpoints for {surface}"
            );
        }
        assert!(surface_actions("collections").len() >= 3);
        assert!(surface_actions("integrations").len() >= 4);
        assert!(surface_actions("identity").len() >= 3);
        assert!(surface_actions("wishlist").len() >= 2);
    }

    #[test]
    fn bulk_workbench_renders_all_surface_catalogs() {
        let html = bulk_workbench_html();
        for surface in surface_names() {
            assert!(
                html.contains(&format!(r#"data-slskr-surface="{surface}""#)),
                "missing workbench surface {surface}"
            );
        }
        for expected in [
            "/api/v0/share-grants",
            "/api/v0/musicbrainz/targets",
            "/api/v0/telemetry/metrics/kpis",
            "/api/v0/source-feeds",
            "/api/v0/soulseek/interests",
            "/api/v0/database/vacuum",
        ] {
            assert!(
                html.contains(expected),
                "missing workbench endpoint {expected}"
            );
        }
        assert!(html.contains("ShareGrant"));
        assert!(html.contains("ShareGroupMember"));
        assert!(html.contains("Permissions"));
    }

    #[test]
    fn every_route_surface_has_bulk_catalog_entries() {
        for surface in surface_names() {
            assert!(
                !surface_route_catalog_html(surface).is_empty(),
                "missing routes for {surface}"
            );
            assert!(
                !surface_endpoint_catalog_html(surface).is_empty(),
                "missing endpoints for {surface}"
            );
        }
    }

    #[test]
    fn rust_route_inventory_matches_current_react_route_surface() {
        let route_paths = ui_routes()
            .iter()
            .map(|route| route.path)
            .collect::<Vec<_>>();
        for expected in [
            "/searches",
            "/searches/:id",
            "/discovery-graph",
            "/playlist-intake",
            "/wishlist",
            "/browse",
            "/users",
            "/contacts",
            "/solid",
            "/collections",
            "/sharegroups",
            "/shared",
            "/chat",
            "/pods",
            "/rooms",
            "/messages",
            "/uploads",
            "/downloads",
            "/system",
            "/system/:tab",
        ] {
            assert!(route_paths.contains(&expected), "missing route {expected}");
            assert!(
                REACT_ROUTES.contains(&format!("path=\"{expected}\""))
                    || REACT_ROUTES.contains(&format!("to=\"{expected}\"")),
                "route {expected} is no longer present in the React UI"
            );
        }
    }

    #[test]
    fn rust_nav_inventory_matches_current_react_navigation() {
        let labels = nav_items()
            .iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        for expected in [
            "Search",
            "Discovery Graph",
            "Playlist Intake",
            "Wishlist",
            "Downloads",
            "Uploads",
            "Messages",
            "Users",
            "Contacts",
            "Solid",
            "Collections",
            "Share Groups",
            "Shared with Me",
            "Browse",
            "System",
        ] {
            assert!(labels.contains(&expected), "missing nav item {expected}");
        }
        for item in nav_items() {
            assert!(
                REACT_NAV.contains(&format!("to=\"{}\"", item.href))
                    || REACT_HEADER.contains(&format!("to=\"{}\"", item.href)),
                "nav item {} does not match a React NavLink",
                item.href
            );
        }
    }

    #[test]
    fn api_contract_inventory_covers_core_old_ui_surfaces() {
        let surfaces = api_endpoints()
            .iter()
            .map(|endpoint| endpoint.surface)
            .collect::<Vec<_>>();
        for expected in [
            "application",
            "session",
            "search",
            "wishlist",
            "transfers",
            "rooms",
            "messages",
            "browse",
            "identity",
            "collections",
            "integrations",
            "system",
        ] {
            assert!(
                surfaces.contains(&expected),
                "missing API surface {expected}"
            );
        }
    }

    #[test]
    fn route_actions_cover_core_old_ui_surfaces() {
        let actions = route_actions();
        let surfaces = actions
            .iter()
            .map(|action| action.surface)
            .collect::<Vec<_>>();
        for expected in [
            "search",
            "transfers",
            "rooms",
            "messages",
            "browse",
            "wishlist",
            "identity",
            "collections",
            "integrations",
            "system",
        ] {
            assert!(surfaces.contains(&expected), "missing action {expected}");
        }
        for expected in [
            ("POST", "/searches"),
            ("PUT", "/searches/:id"),
            ("DELETE", "/searches/:id"),
            ("DELETE", "/searches"),
            ("POST", "/transfers/downloads/:username"),
            ("PUT", "/transfers/downloads/accelerated"),
            ("POST", "/rooms/joined"),
            ("POST", "/rooms/joined/:roomName/messages"),
            ("DELETE", "/rooms/joined/:roomName"),
            ("POST", "/conversations/:username"),
            ("PUT", "/conversations/:username"),
            ("DELETE", "/conversations/:username"),
            ("POST", "/users/:username/directory"),
            ("POST", "/wishlist"),
            ("POST", "/wishlist/:id/search"),
            ("POST", "/contacts"),
            ("POST", "/contacts/from-discovery"),
            ("POST", "/contacts/from-invite"),
            ("POST", "/users/watch"),
            ("POST", "/users/notes"),
            ("POST", "/collections"),
            ("POST", "/sharegroups"),
            ("POST", "/sharegroups/:id/members"),
            ("POST", "/share-grants"),
            ("PUT", "/share-grants/:id"),
            ("POST", "/share-grants/:id/backfill"),
            ("POST", "/share-grants/:id/token"),
            ("DELETE", "/share-grants/:id"),
            ("POST", "/library/items"),
            ("POST", "/source-feed-imports/preview"),
            ("POST", "/discovery-graph"),
            ("POST", "/source-feeds"),
            ("POST", "/musicbrainz/targets"),
            ("POST", "/musicbrainz/release-radar/subscriptions"),
            ("POST", "/songid/runs"),
            ("POST", "/jobs/discography"),
            ("POST", "/shares/rescan"),
            ("POST", "/database/vacuum"),
        ] {
            assert!(
                actions
                    .iter()
                    .any(|action| action.method == expected.0 && action.path == expected.1),
                "missing action {} {}",
                expected.0,
                expected.1
            );
        }
    }
}
