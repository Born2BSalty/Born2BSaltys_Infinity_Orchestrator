use super::{planned_eet_bgee_path, render_eet_planned_paths};
use crate::app::state::Step1State;

fn plan() -> Step1State {
    Step1State {
        game_install: "EET".to_string(),
        new_pre_eet_dir_enabled: true,
        eet_pre_dir: "  planned/Pré EET 目录  ".to_string(),
        eet_bgee_game_folder: "original/BGEE".to_string(),
        bgee_game_folder: "other/BGEE".to_string(),
        eet_bg2ee_game_folder: "source/BG2EE".to_string(),
        eet_new_dir: "target/BG2EE".to_string(),
        ..Step1State::default()
    }
}

#[test]
fn planned_preview_uses_selected_plan_without_filesystem_or_fallback() {
    let mut state = plan();
    assert_eq!(planned_eet_bgee_path(&state), Some("planned/Pré EET 目录"));
    state.new_pre_eet_dir_enabled = false;
    assert_eq!(planned_eet_bgee_path(&state), Some("original/BGEE"));
    state.eet_bgee_game_folder = " \t ".to_string();
    assert_eq!(planned_eet_bgee_path(&state), None);
    state.new_pre_eet_dir_enabled = true;
    state.eet_pre_dir.clear();
    assert_eq!(planned_eet_bgee_path(&state), None);
    state.eet_pre_dir = "planned/new".to_string();
    state.game_install = "BG2EE".to_string();
    assert_eq!(planned_eet_bgee_path(&state), None);
}

#[test]
fn planned_preview_preserves_translation_options_and_exact_token_scope() {
    let raw = "Répertoire détecté : %bgee_dir% ?\n1 = Oui\n2 = Non\n%bgee_dir% / %bg2ee_dir% / %BGEE_DIR%";
    let state = plan();
    for identity in ["EET.tp2", "setup-eet.tp2", "EET/EET.TP2"] {
        assert_eq!(
            render_eet_planned_paths(raw, identity, planned_eet_bgee_path(&state)),
            "Répertoire détecté : planned/Pré EET 目录 ?\n1 = Oui\n2 = Non\nplanned/Pré EET 目录 / %bg2ee_dir% / %BGEE_DIR%"
        );
    }
    for identity in ["other.tp2", "EET_END.tp2", ""] {
        assert_eq!(
            render_eet_planned_paths(raw, identity, Some("planned")),
            raw
        );
    }
    assert_eq!(render_eet_planned_paths(raw, "EET.tp2", None), raw);
    assert_eq!(
        render_eet_planned_paths("No variable", "EET.tp2", Some("planned")),
        "No variable"
    );
    assert_eq!(
        raw,
        "Répertoire détecté : %bgee_dir% ?\n1 = Oui\n2 = Non\n%bgee_dir% / %bg2ee_dir% / %BGEE_DIR%"
    );
}
