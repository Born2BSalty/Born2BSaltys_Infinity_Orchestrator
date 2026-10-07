// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HelpPage {
    Step2 { created_from_mods: bool },
    Step3,
    Step4,
    Step5,
    Downloading { manual_downloads: bool },
    Installing,
}

pub(crate) struct HelpBullet {
    pub lead: Option<&'static str>,
    pub body: &'static str,
}

pub(crate) struct HelpText {
    pub title: &'static str,
    pub bullets: Vec<HelpBullet>,
}

const fn bullet(body: &'static str) -> HelpBullet {
    HelpBullet { lead: None, body }
}

const fn led_bullet(lead: &'static str, body: &'static str) -> HelpBullet {
    HelpBullet {
        lead: Some(lead),
        body,
    }
}

#[must_use]
pub(crate) fn help_text(page: HelpPage) -> HelpText {
    match page {
        HelpPage::Step2 { created_from_mods } => step2_text(created_from_mods),
        HelpPage::Step3 => step3_text(),
        HelpPage::Step4 => step4_text(),
        HelpPage::Step5 => step5_text(),
        HelpPage::Downloading { manual_downloads } => downloading_text(manual_downloads),
        HelpPage::Installing => installing_text(),
    }
}

fn step2_text(created_from_mods: bool) -> HelpText {
    let mut bullets = vec![
        bullet(
            "Tick the mods and components you want. Open a mod with its arrow to see its \
             components; the counter on the right shows how many are ticked for this game.",
        ),
        bullet(
            "Click the ? while hovering over a mod or component to read about it in the \
             Details panel.",
        ),
        bullet(
            "The Versions\u{2026} button allows you to check for updates to your mods or pin \
             the mod source to another fork, commit hash, release tag, branch, etc. (This is \
             an advanced feature, so feel free to ask for help on the BIO Discord.)",
        ),
        bullet(
            "A yellow PROMPT pill means the component asks questions during install; a red \
             pill is a compatibility problem. Click either to see why.",
        ),
    ];
    if created_from_mods {
        bullets.push(bullet(
            "Each game tab keeps its own selection. \"Select <game> via WeiDU Log\" ticks \
             everything listed in an existing weidu.log.",
        ));
        bullets.push(bullet(
            "To learn how to add mods, use the \"?\" next to Rescan Mods.",
        ));
    }
    HelpText {
        title: "Choose what to install",
        bullets,
    }
}

fn step3_text() -> HelpText {
    HelpText {
        title: "Set the install order",
        bullets: vec![
            bullet(
                "Mods install from the top of this list to the bottom. Drag a component to \
                 move it; drag a mod's header to move the whole mod.",
            ),
            bullet(
                "Click, Ctrl-click and Shift-click select several rows. Right-click, then \
                 \"Move selection to top\" or \"Move selection to bottom\" sends them to \
                 either end.",
            ),
            led_bullet(
                "This is where you make an install fully automatic.",
                "A yellow PROMPT pill marks a component that asks questions during install; \
                 click the pill to see them. Right-click the component \u{2192} \"Set \
                 @wlb-inputs\u{2026}\" and type the answers in the order they are asked, \
                 separated by commas (for example y,2,n). BIO types them for you during the \
                 install, so it never stops to wait. \"Clear Prompt Data\" removes them.",
            ),
            bullet(
                "The padlock on a mod header holds that mod in place. Undo and Redo step \
                 through your changes; Collapse All and Expand All fold the list.",
            ),
            bullet(
                "Right-click a component to uncheck it back in Step 2. Right-click a mod \
                 header \u{2192} \"Clone Parent\" makes a second, empty header you can drag \
                 components into, to split one mod across two places in the order.",
            ),
            bullet(
                "The counts on the game tabs are compatibility conflicts and issues; click a \
                 pill on a row to read the rule. Conflicts must be cleared before the install \
                 will start.",
            ),
        ],
    }
}

fn step4_text() -> HelpText {
    HelpText {
        title: "Review",
        bullets: vec![
            bullet(
                "This is the final order, exactly as it will be written to each game's \
                 weidu.log. Nothing here is editable.",
            ),
            bullet(
                "Switch game tabs to check each game. \"Save weidu.log\" writes the file or \
                 files out if you want a copy.",
            ),
            bullet("Something wrong? Go back with Previous and fix it in Step 2 or Step 3."),
        ],
    }
}

fn step5_text() -> HelpText {
    HelpText {
        title: "Install",
        bullets: vec![
            bullet(
                "Install starts the run: BIO prepares the install folder, then runs each mod \
                 in order. The console shows WeiDU's live output.",
            ),
            bullet(
                "If a mod asks a question, type the answer in the box under the console. \
                 NOTE: we recommend pre-inputting your answers in Step 3 so your install \
                 won't get interrupted to wait for your inputs.",
            ),
            bullet(
                "General, Important Only and Installed Only filter the console; Auto-scroll \
                 follows the newest line. The Actions menu copies or saves the console log and \
                 opens the logs folder.",
            ),
            bullet(
                "Cancel stops the run. After a failure the button becomes Resume and picks up \
                 where it stopped.",
            ),
            bullet(
                "When it finishes, Open game folder shows the modded game and Return to \
                 Home goes back to your lists.",
            ),
        ],
    }
}

fn downloading_text(manual_downloads: bool) -> HelpText {
    let mut bullets = vec![
        bullet(
            "BIO first checks your Mods archive folder for archives it already has, \
             downloads the rest, then extracts everything. When extraction finishes it \
             moves on by itself.",
        ),
        bullet(
            "Each row is one mod archive with its status. The bar at the top is the \
             overall progress.",
        ),
    ];
    if manual_downloads {
        bullets.extend(manual_download_bullets());
    }
    bullets.push(bullet(
        "Archives already downloaded stay in your Mods archive folder, so a cancelled \
         run does not download them again.",
    ));
    HelpText {
        title: "What's happening while BIO downloads",
        bullets,
    }
}

fn manual_download_bullets() -> Vec<HelpBullet> {
    vec![
        bullet(
            "A Manual downloads box appears when BIO cannot fetch a mod itself, such as a \
                 Nexus Mods page or a mod with no download source. Open page opens the mod's \
                 download page; save the file into your Mods archive folder and BIO finds it \
                 within a few seconds, or use Pick file\u{2026} to point at it.",
        ),
        bullet(
            "Everything else keeps downloading while you do that. Only extraction waits \
                 for the manual mods.",
        ),
        bullet(
            "\"Continue without\" skips the missing mods: the rest is extracted and the \
                 list opens on Mods / Components so you can decide what to do. Anything that \
                 depends on a skipped mod may fail.",
        ),
    ]
}

fn installing_text() -> HelpText {
    HelpText {
        title: "What's happening during the install",
        bullets: vec![
            bullet(
                "BIO works through this list by itself, installing the mods one at a time in \
                 the list's order.",
            ),
            bullet(
                "The console is WeiDU's live output; the progress line above it shows which \
                 mod is installing and how many are left.",
            ),
            bullet(
                "If a mod asks a question BIO has no saved answer for, the install waits. \
                 Type the answer in the box under the console and press Enter.",
            ),
            bullet(
                "Cancel stops the run. After a stop or a failure the button becomes Resume and \
                 continues from the same mod. Leave the game folder alone while a run is \
                 active.",
            ),
            bullet("When it finishes, Open game folder shows the modded game."),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{HelpPage, help_text};

    #[test]
    fn all_six_pages_return_non_empty_text() {
        let pages = [
            HelpPage::Step2 {
                created_from_mods: false,
            },
            HelpPage::Step2 {
                created_from_mods: true,
            },
            HelpPage::Step3,
            HelpPage::Step4,
            HelpPage::Step5,
            HelpPage::Downloading {
                manual_downloads: true,
            },
            HelpPage::Downloading {
                manual_downloads: false,
            },
            HelpPage::Installing,
        ];
        for page in pages {
            let text = help_text(page);
            assert_ne!(text.title.len(), 0);
            assert_ne!(text.bullets.len(), 0);
        }
    }

    #[test]
    fn step2_bullet_count_depends_on_created_from_mods() {
        let modify_fork = help_text(HelpPage::Step2 {
            created_from_mods: false,
        });
        assert_eq!(modify_fork.bullets.len(), 4);

        let from_mods = help_text(HelpPage::Step2 {
            created_from_mods: true,
        });
        assert_eq!(from_mods.bullets.len(), 6);
    }

    #[test]
    fn downloading_mentions_manual_downloads_only_where_the_box_can_appear() {
        let with_box = help_text(HelpPage::Downloading {
            manual_downloads: true,
        });
        let without_box = help_text(HelpPage::Downloading {
            manual_downloads: false,
        });
        assert_eq!(with_box.bullets.len(), 6);
        assert_eq!(without_box.bullets.len(), 3);
        assert!(
            without_box
                .bullets
                .iter()
                .all(|bullet| !bullet.body.contains("Manual downloads")
                    && !bullet.body.contains("Continue without"))
        );
    }

    #[test]
    fn step3_has_the_prompt_inputs_copy_and_one_lead() {
        let text = help_text(HelpPage::Step3);
        let joined: String = text.bullets.iter().map(|b| b.body).collect();
        assert!(joined.contains("Set @wlb-inputs"));
        assert!(joined.contains("separated by commas"));
        let leads = text.bullets.iter().filter(|b| b.lead.is_some()).count();
        assert_eq!(leads, 1);
    }

    #[test]
    fn no_page_mentions_edit_prompt_json() {
        let pages = [
            HelpPage::Step2 {
                created_from_mods: true,
            },
            HelpPage::Step3,
            HelpPage::Step4,
            HelpPage::Step5,
            HelpPage::Downloading {
                manual_downloads: true,
            },
            HelpPage::Downloading {
                manual_downloads: false,
            },
            HelpPage::Installing,
        ];
        for page in pages {
            let text = help_text(page);
            assert!(!text.title.contains("Edit Prompt JSON"));
            for b in &text.bullets {
                assert!(!b.body.contains("Edit Prompt JSON"));
                if let Some(lead) = b.lead {
                    assert!(!lead.contains("Edit Prompt JSON"));
                }
            }
        }
    }
}
