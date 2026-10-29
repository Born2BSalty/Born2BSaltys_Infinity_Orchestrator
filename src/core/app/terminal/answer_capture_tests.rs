use crate::app::terminal::{EmbeddedTerminal, input::write_line_to};
use std::io::{self, Write};

struct FailingWriter {
    fail_flush: bool,
}

impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.fail_flush {
            Ok(bytes.len())
        } else {
            Err(io::Error::other("fixture write failed"))
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fixture flush failed"))
    }
}

#[test]
fn manual_wlb_checked_writer_preserves_blank_unicode_and_reports_both_errors() {
    let mut bytes = Vec::new();
    write_line_to(&mut bytes, "").expect("blank send");
    write_line_to(&mut bytes, "世界,a").expect("Unicode send");
    assert_eq!(
        String::from_utf8(bytes)
            .expect("UTF8")
            .replace("\r\n", "\n"),
        "\n世界,a\n"
    );
    assert!(write_line_to(&mut FailingWriter { fail_flush: false }, "x").is_err());
    assert!(write_line_to(&mut FailingWriter { fail_flush: true }, "x").is_err());
}

#[test]
fn manual_wlb_checked_send_without_child_is_error_and_records_nothing() {
    let mut terminal = EmbeddedTerminal::new().expect("terminal without process");
    assert!(terminal.try_send_line("answer").is_err());
    assert!(terminal.take_confirmed_answers().is_empty());
    assert!(terminal.current_capture_occurrence().is_none());
}

#[test]
fn manual_wlb_terminal_reconstructs_chunks_preserves_carry_on_clear_and_drains_once() {
    use crate::app::game_authority::GameSlot;
    use crate::app::step5::answer_capture::{
        AnswerOrigin, CaptureComponentKey, CaptureRunKey, CaptureSession,
    };
    use crate::app::wlb_inputs::parse_identity;
    let component = CaptureComponentKey {
        slot: GameSlot::First,
        identity: parse_identity("~MOD/MOD.TP2~ #2 #17 // Label").expect("identity"),
    };
    let session = CaptureSession::new(
        CaptureRunKey {
            run_id: 4,
            workspace_id: None,
            generation: 8,
            game_install: "BGEE".to_string(),
        },
        vec![component],
    );
    let mut terminal = EmbeddedTerminal::new().expect("terminal");
    terminal.set_capture_session(session);
    terminal.update_important_lines("[INFO mod_installer::installers] Installing mod Component { tp_file: \"MOD/MOD.TP2\", component: \"17\", lang: \"2\", component_name: \"Label\" }\nUser input req");
    assert!(terminal.current_capture_occurrence().is_none());
    terminal.clear_console();
    terminal.update_important_lines("uired\nQuestion is:\nWhat ");
    assert!(terminal.current_capture_occurrence().is_none());
    terminal.update_important_lines("choice?\n");
    let occurrence = terminal
        .current_capture_occurrence()
        .expect("complete reconstructed question");
    assert_eq!(occurrence.block_key, "What choice?");
    assert!(
        terminal
            .record_capture_sent(&occurrence, "answer", AnswerOrigin::Manual)
            .expect("sent")
    );
    terminal.update_important_lines("SUCCESSFULLY INSTAL");
    assert!(terminal.take_confirmed_answers().is_empty());
    terminal.update_important_lines("LED Label\n");
    terminal.invalidate_capture();
    assert_eq!(terminal.take_confirmed_answers()[0].answers, ["answer"]);
    assert!(terminal.take_confirmed_answers().is_empty());
}

fn replay_fixture(phase: bool, language_collision: bool) -> EmbeddedTerminal {
    use crate::app::game_authority::GameSlot;
    use crate::app::step5::answer_capture::{CaptureComponentKey, CaptureRunKey, CaptureSession};
    use crate::app::step5::scripted_inputs::load_from_logs;
    use crate::app::wlb_inputs::parse_identity;
    let raw =
        "~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: first,second\n~MOD/MOD.TP2~ #2 #18 // Other";
    let mut components = vec![17, 18]
        .into_iter()
        .map(|id| CaptureComponentKey {
            slot: GameSlot::First,
            identity: parse_identity(&format!("~MOD/MOD.TP2~ #2 #{id} // Label"))
                .expect("identity"),
        })
        .collect::<Vec<_>>();
    let mut log = raw.to_string();
    if language_collision {
        components.push(CaptureComponentKey {
            slot: GameSlot::First,
            identity: parse_identity("~MOD/MOD.TP2~ #0 #17 // Collision").expect("identity"),
        });
        log.push_str("\n~MOD/MOD.TP2~ #0 #17 // Collision // @wlb-inputs: wrong");
    }
    let plan = load_from_logs(
        &[(GameSlot::First, log)],
        vec![("mod.tp2#18".to_string(), vec!["fallback".to_string()])],
    )
    .expect("plan");
    let mut terminal = EmbeddedTerminal::new().expect("terminal");
    terminal.set_scripted_inputs(plan);
    terminal.set_capture_session(CaptureSession::new(
        CaptureRunKey {
            run_id: 1,
            workspace_id: None,
            generation: 1,
            game_install: "EET".to_string(),
        },
        components,
    ));
    if phase {
        terminal.update_important_lines("Beginning pre EET install process\n");
    }
    terminal.update_important_lines(concat!(
        "[INFO mod_installer::installers] Installing mod WeiduBatchedComponents [Component { tp_file: \"MOD/MOD.TP2\", component: \"17\", lang: \"2\", component_name: \"Label\" }, ",
        "Component { tp_file: \"MOD/MOD.TP2\", component: \"18\", lang: \"2\", component_name: \"Other\" }]\n",
        "User input required\nQuestion is:\nChoice?\n"
    ));
    terminal.current_component_key = Some("mod.tp2#17".to_string());
    terminal.current_component_tp2 = Some("mod.tp2".to_string());
    terminal.current_component_id = Some("17".to_string());
    terminal.current_component_name = Some("Label".to_string());
    terminal
}

#[test]
fn manual_wlb_batch_replay_uses_explicit_current_context_without_capture_owner() {
    let mut terminal = replay_fixture(true, false);
    assert!(terminal.current_capture_occurrence().is_none());
    assert_eq!(
        terminal.peek_next_scripted_input_for_current(),
        Some("first")
    );
    assert_eq!(
        terminal.take_next_scripted_input_for_current().as_deref(),
        Some("first")
    );
    assert_eq!(
        terminal.peek_next_scripted_input_for_current(),
        Some("second")
    );
    assert!(terminal.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_batch_unrelated_fallback_survives_another_typed_queue() {
    let mut terminal = replay_fixture(true, false);
    terminal.current_component_key = Some("mod.tp2#18".to_string());
    terminal.current_component_id = Some("18".to_string());
    terminal.current_component_name = Some("Other".to_string());
    assert_eq!(
        terminal.peek_next_scripted_input_for_current(),
        Some("fallback")
    );
    assert_eq!(
        terminal.take_next_scripted_input_for_current().as_deref(),
        Some("fallback")
    );
    assert_eq!(terminal.peek_next_scripted_input_for_current(), None);
}

#[test]
fn manual_wlb_batch_failed_write_preserves_typed_queue() {
    let mut terminal = replay_fixture(true, false);
    assert_eq!(
        terminal.peek_next_scripted_input_for_current(),
        Some("first")
    );
    assert!(terminal.try_send_line("first").is_err());
    assert_eq!(
        terminal.peek_next_scripted_input_for_current(),
        Some("first")
    );
    assert!(terminal.current_capture_occurrence().is_none());
    assert!(terminal.take_confirmed_answers().is_empty());
}

#[test]
fn manual_wlb_batch_unknown_phase_and_language_collision_do_not_guess() {
    let mut unknown = replay_fixture(false, false);
    assert_eq!(unknown.peek_next_scripted_input_for_current(), None);
    assert_eq!(unknown.take_next_scripted_input_for_current(), None);
    let mut collision = replay_fixture(true, true);
    assert_eq!(collision.peek_next_scripted_input_for_current(), None);
    assert_eq!(collision.take_next_scripted_input_for_current(), None);
}
