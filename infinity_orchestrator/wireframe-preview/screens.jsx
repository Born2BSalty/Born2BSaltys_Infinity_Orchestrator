/* global React */
const { useState } = React;

// ---------- shared sketchy primitives ----------
const sketchyBorder = {
  border: "1.5px solid var(--border-strong)",
  borderRadius: "3px",
  background: "var(--shell-bg)",
};

const Box = ({ children, style, onClick, label }) => (
  <div
    className="sk-box"
    onClick={onClick}
    style={{ ...sketchyBorder, padding: "10px 12px", position: "relative", ...style }}
  >
    {label && <span className="sk-corner-label">{label}</span>}
    {children}
  </div>
);

const Btn = ({ children, primary, small, style, onClick, disabled, title }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    title={title}
    className="sk-btn"
    style={{
      ...sketchyBorder,
      background: primary ? "var(--accent)" : "var(--shell-bg)",
      color: primary ? "#1a2638" : "var(--text)",
      padding: small ? "4px 10px" : "8px 16px",
      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
      fontSize: small ? "12px" : "14px",
      cursor: disabled ? "not-allowed" : "pointer",
      opacity: disabled ? 0.5 : undefined,
      boxShadow: primary ? "2px 2px 0 var(--shadow)" : "none",
      ...style,
    }}
  >
    {children}
  </button>
);

const Input = ({ placeholder, value, mono, style, wide }) => (
  <div
    style={{
      ...sketchyBorder,
      padding: "6px 10px",
      background: "var(--input-bg)",
      fontFamily: mono ? "'Poppins', 'FiraCode Nerd', sans-serif" : "'Poppins', 'FiraCode Nerd', sans-serif",
      fontSize: mono ? "12px" : "16px",
      color: value ? "var(--text)" : "var(--text-faint)",
      minHeight: "22px",
      width: wide ? "100%" : "auto",
      boxSizing: "border-box",
      ...style,
    }}
  >
    {value || placeholder}
  </div>
);

const Toggle = ({ on }) => (
  <div
    style={{
      ...sketchyBorder,
      width: "38px",
      height: "20px",
      borderRadius: "12px",
      background: on ? "var(--accent)" : "var(--input-bg)",
      position: "relative",
      flexShrink: 0,
    }}
  >
    <div
      style={{
        position: "absolute",
        top: "1px",
        left: on ? "19px" : "1px",
        width: "16px",
        height: "16px",
        borderRadius: "50%",
        background: "var(--border-strong)",
        transition: "left 0.15s",
      }}
    />
  </div>
);

const FolderInput = ({ label, placeholder, hint, value, onBrowse }) => (
  <div>
    <Label hand style={{ marginBottom: 4, color: "var(--text-muted)" }}>{label}</Label>
    <div style={{ display: "flex", gap: 8, alignItems: "stretch" }}>
      <Input
        wide
        mono
        placeholder={placeholder}
        value={value}
        style={{ flex: 1, fontSize: 13, padding: "8px 12px" }}
      />
      <button
        className="sk-btn"
        onClick={onBrowse}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          color: "var(--text)",
          fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
          fontSize: 14,
          padding: "6px 14px",
          cursor: "pointer",
          whiteSpace: "nowrap",
        }}
      >
        browse...
      </button>
    </div>
    {hint && <Label style={{ marginTop: 4, color: "var(--text-faint)", fontSize: 13 }}>{hint}</Label>}
  </div>
);

const DestinationNotEmptyWarning = ({ choice, setChoice, allowPartial = true }) => {
  const options = [
    { id: "clear", label: "Clear contents" },
    { id: "backup", label: "Backup contents then proceed" },
    ...(allowPartial ? [{ id: "continue", label: "Continue partial installation" }] : []),
  ];
  return (
    <div style={{
      border: "1.5px solid #edc547",
      borderRadius: "3px",
      background: "rgba(237, 197, 71, 0.18)",
      boxShadow: "2px 2px 0 var(--shadow)",
      padding: "10px 14px",
      marginTop: 12,
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
        <span style={{ fontSize: 13, lineHeight: 1 }}>⚠</span>
        <Label style={{ fontWeight: 500, fontSize: 13 }}>Target directory not empty</Label>
      </div>
      <Label hand style={{ fontSize: 14, color: "var(--text-muted)", marginBottom: 10 }}>
        How would you like to proceed?
      </Label>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {options.map((o) => (
          <Btn key={o.id} small primary={choice === o.id} onClick={() => setChoice(o.id)}>
            {o.label}
          </Btn>
        ))}
      </div>
    </div>
  );
};

const Placeholder = ({ label, h = 80, style }) => (
  <div
    style={{
      ...sketchyBorder,
      height: h,
      background:
        "repeating-linear-gradient(45deg, var(--shell-bg) 0 8px, var(--border-soft) 8px 9px)",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
      fontSize: "11px",
      color: "var(--text-faint)",
      letterSpacing: "0.5px",
      ...style,
    }}
  >
    {label}
  </div>
);

const Label = ({ children, hand, style, ...rest }) => (
  <div
    {...rest}
    style={{
      fontFamily: hand ? "'Poppins', 'FiraCode Nerd', sans-serif" : "'Poppins', 'FiraCode Nerd', sans-serif",
      fontSize: hand ? "14px" : "13px",
      color: hand ? "var(--accent-deep)" : "var(--text)",
      ...style,
    }}
  >
    {children}
  </div>
);

const ScreenTitle = ({ title, sub }) => (
  <div style={{ marginBottom: "20px" }}>
    <h1
      style={{
        fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
        fontSize: "22px",
        margin: 0,
        fontWeight: 500,
        color: "var(--text)",
        lineHeight: 1,
      }}
    >
      {title}
    </h1>
    {sub && (
      <div
        style={{
          fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
          fontSize: "13px",
          color: "var(--text-muted)",
          marginTop: "4px",
        }}
      >
        {sub}
      </div>
    )}
  </div>
);

// ---------- HOME ----------
// Mock in-progress builds: started or partially-installed modlists not yet successfully finished.
// They show up on Home and in the Load Draft dialog.
const MOCK_IN_PROGRESS_BUILDS = [
  { id: "tactical-eet-2026", n: "Tactical EET 2026", meta: "9 mods · 136 components · last touched 2 hours ago · paused at Step 3", game: "EET" },
  { id: "polished-bg2ee", n: "Polished BG2EE", meta: "22 mods · 47 components · saved as draft yesterday · paused at Step 4", game: "BG2EE" },
];

const HomeScreen = ({ v1, navigate, resumeBuild, reinstallBuild }) => {
  const [toast, setToast] = useState("");
  const [deleteTarget, setDeleteTarget] = useState(null);
  const [reinstallTarget, setReinstallTarget] = useState(null);
  const copyPasteCode = (modlistName) => {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(SAMPLE_PASTE_CODE).catch(() => {});
    }
    setToast(`Copied import code for "${modlistName}"`);
    setTimeout(() => setToast(""), 1800);
  };
  const requestDelete = (m) => setDeleteTarget(m);
  const confirmDelete = () => {
    const name = deleteTarget && deleteTarget.n;
    setDeleteTarget(null);
    setToast(`Deleted "${name}"`);
    setTimeout(() => setToast(""), 1800);
  };
  const requestReinstall = (m) => setReinstallTarget(m);
  const confirmReinstall = () => {
    const target = reinstallTarget;
    setReinstallTarget(null);
    if (target && reinstallBuild) {
      reinstallBuild({ ...target, destination: sampleDestFor(target) });
    }
  };
  const sampleDestFor = (m) => {
    const slug = (m && m.n ? m.n : "modlist").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
    return `C:\\BIO\\modlists\\${slug}`;
  };
  const inProgress = MOCK_IN_PROGRESS_BUILDS;
  const finished = [
    { n: "Tactics & Tweaks · BG2EE", meta: "47 mods · 2.3 GB · installed 2 days ago" },
    { n: "EET Megamod · 2026", meta: "120 mods · 8.1 GB · installed last week" },
    { n: "IWD Polished", meta: "32 mods · 1.4 GB · installed last month" },
  ];
  const installedItems = finished.map((m) => ({ ...m, state: "installed" }));
  const inProgressItems = inProgress.map((m) => ({ ...m, state: "in-progress" }));
  const allItems = [...installedItems, ...inProgressItems];
  const defaultFilter = installedItems.length > 0 ? "installed" : inProgressItems.length > 0 ? "in-progress" : "all";
  const [filter, setFilter] = useState(defaultFilter);
  const visible = filter === "all" ? allItems : allItems.filter((m) => m.state === filter);
  const subSummary = [
    `${finished.length} modlists installed`,
    inProgress.length > 0 ? `${inProgress.length} in progress` : null,
    "last played BG2EE 2 days ago",
  ].filter(Boolean).join(" · ");

  // Chip-style filter button — lighter than the regular primary button (no drop shadow, rounded ends).
  const Chip = ({ active, count, onClick, children }) => (
    <button
      onClick={onClick}
      className="sk-btn"
      style={{
        ...sketchyBorder,
        background: active ? "var(--accent)" : "var(--shell-bg)",
        color: active ? "#1a2638" : "var(--text)",
        padding: "4px 12px",
        fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
        fontSize: 13,
        fontWeight: active ? 500 : 400,
        cursor: "pointer",
        borderRadius: 14,
        boxShadow: "none",
      }}
    >
      {children} <span style={{ color: active ? "#1a2638" : "var(--text-faint)", fontWeight: 400 }}>({count})</span>
    </button>
  );

  return (
  <div className="sk-page">
    <ScreenTitle title="Welcome back, adventurer" sub={subSummary} />

    <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: "20px", marginBottom: "20px" }}>
      <Box style={{ padding: "16px" }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center", flexWrap: "wrap" }}>
          <Chip active={filter === "installed"} count={installedItems.length} onClick={() => setFilter("installed")}>Installed</Chip>
          {inProgressItems.length > 0 && (
            <Chip active={filter === "in-progress"} count={inProgressItems.length} onClick={() => setFilter("in-progress")}>In progress</Chip>
          )}
          <Chip active={filter === "all"} count={allItems.length} onClick={() => setFilter("all")}>All</Chip>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          {visible.length === 0 ? (
            <Label style={{ color: "var(--text-faint)" }}>
              {filter === "installed" ? "No installed modlists yet. Create one or paste an import code to add the first." :
               filter === "in-progress" ? "No in-progress builds. Start a new modlist from \"create your own\"." :
               "No modlists yet."}
            </Label>
          ) : visible.map((m) => (
            <Box key={m.id || m.n} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 12px" }}>
              <div>
                <Label>{m.n}</Label>
                <Label hand style={{ fontSize: "14px", color: "var(--text-faint)" }}>{m.meta}</Label>
              </div>
              {m.state === "in-progress" ? (
                <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
                  <Btn small primary onClick={() => resumeBuild && resumeBuild(m)}>resume</Btn>
                  <Kebab items={[
                    { label: "Copy import code", onClick: () => copyPasteCode(m.n) },
                    { label: "Rename", onClick: () => {} },
                    { label: "Delete", onClick: () => requestDelete(m) },
                  ]} />
                </div>
              ) : (
                <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
                  <Btn small>play</Btn>
                  <Kebab items={[
                    { label: "Copy import code", onClick: () => copyPasteCode(m.n) },
                    { label: "Open install folder", onClick: () => {} },
                    { label: "Rename", onClick: () => {} },
                    { label: "Reinstall", onClick: () => requestReinstall(m) },
                    { label: "Delete", onClick: () => requestDelete(m) },
                  ]} />
                </div>
              )}
            </Box>
          ))}
        </div>
      </Box>

      <Box label="add a modlist" style={{ padding: "16px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          {!v1 && <Btn onClick={() => navigate && navigate("explore")}>Browse community modlists</Btn>}
          <Btn primary onClick={() => navigate && navigate("install")}>paste import code</Btn>
          <Btn onClick={() => navigate && navigate("create")}>create your own</Btn>
        </div>
        <div style={{ marginTop: "20px" }}>
          <Label hand>game installs detected</Label>
          <div style={{ display: "flex", flexDirection: "column", gap: "4px", marginTop: "6px" }}>
            <Label style={{ fontSize: "14px" }}>✓ BGEE</Label>
            <Label style={{ fontSize: "14px" }}>✓ BG2EE</Label>
            <Label style={{ fontSize: "14px", color: "var(--text-faint)" }}>? IWDEE · not found</Label>
          </div>
        </div>
      </Box>
    </div>

    {toast && (
      <div style={{
        position: "fixed",
        bottom: 38,
        left: "50%",
        transform: "translateX(-50%)",
        ...sketchyBorder,
        background: "var(--shell-bg)",
        boxShadow: "3px 3px 0 var(--shadow)",
        padding: "8px 16px",
        fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
        fontSize: 13,
        color: "var(--success)",
        zIndex: 200,
      }}>✓ {toast}</div>
    )}

    <ConfirmDialog
      open={!!deleteTarget}
      danger
      title={deleteTarget ? `Delete "${deleteTarget.n}"?` : ""}
      message={deleteTarget ? (
        <>
          This will permanently remove:
          <ul style={{ margin: "8px 0 8px 18px", padding: 0, color: "var(--text-muted)" }}>
            <li>the modlist's registry entry (it disappears from Home)</li>
            <li>the install folder on disk:
              {" "}<span style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 12, color: "var(--text)" }}>{sampleDestFor(deleteTarget)}</span>
            </li>
          </ul>
          This action cannot be undone.
        </>
      ) : ""}
      confirmLabel="Delete"
      onConfirm={confirmDelete}
      onCancel={() => setDeleteTarget(null)}
    />

    <ConfirmDialog
      open={!!reinstallTarget}
      danger
      title={reinstallTarget ? `Reinstall "${reinstallTarget.n}"?` : ""}
      message={reinstallTarget ? (
        <>
          This will erase the current install folder and re-run the entire install from scratch. Your component selection and order are preserved; the modlist moves back to <strong>in-progress</strong> while the install runs, then returns to installed when complete.
          <ul style={{ margin: "8px 0 8px 18px", padding: 0, color: "var(--text-muted)" }}>
            <li>existing files at:
              {" "}<span style={{ fontFamily: "'JetBrains Mono', monospace", fontSize: 12, color: "var(--text)" }}>{sampleDestFor(reinstallTarget)}</span>
              {" "}will be deleted
            </li>
          </ul>
          This action cannot be undone.
        </>
      ) : ""}
      confirmLabel="Reinstall"
      onConfirm={confirmReinstall}
      onCancel={() => setReinstallTarget(null)}
    />
  </div>
  );
};

// ---------- EXPLORE ----------
const ExploreScreen = () => (
  <div className="sk-page">
    <ScreenTitle title="Explore modlists" sub="curated by the community" />

    <div style={{ display: "flex", gap: "10px", marginBottom: "16px", alignItems: "center" }}>
      <Input placeholder="search modlists..." style={{ flex: 1 }} />
      <Btn small>game ▾</Btn>
      <Btn small>difficulty ▾</Btn>
      <Btn small>size ▾</Btn>
      <Btn small>sort: popular ▾</Btn>
    </div>

    <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "14px" }}>
      {[
        { t: "Tactics & Tweaks", g: "BG2EE", n: "47 mods", a: "by @cernd", d: "harder combat, balanced loot" },
        { t: "EET Megamod 2026", g: "EET", n: "120+ mods", a: "by @sarevok", d: "the kitchen-sink playthrough" },
        { t: "Polished Vanilla+", g: "BGEE", n: "18 mods", a: "by @imoen", d: "QoL fixes, no rebalance" },
        { t: "Heart of Winter Plus", g: "IWDEE", n: "32 mods", a: "by @hjollder", d: "expanded story content" },
        { t: "Sword Coast Stories", g: "BGEE", n: "65 mods", a: "by @candlekeep", d: "RP-focused, new quests" },
        { t: "Athkatla Nights", g: "BG2EE", n: "54 mods", a: "by @jaheira", d: "city overhaul + romance" },
      ].map((m, i) => (
        <Box key={i} style={{ padding: 0, overflow: "hidden", display: "flex", flexDirection: "column" }}>
          <Placeholder label="cover art" h={90} style={{ borderTop: "none", borderLeft: "none", borderRight: "none", borderRadius: 0 }} />
          <div style={{ padding: "10px 12px", display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
            <Label hand style={{ fontSize: "20px", color: "var(--text)" }}>{m.t}</Label>
            <Label style={{ fontSize: "13px", color: "var(--text-muted)" }}>{m.g} · {m.n} · {m.a}</Label>
            <Label style={{ fontSize: "14px", color: "var(--text)", flex: 1 }}>{m.d}</Label>
            <div style={{ display: "flex", gap: "6px", marginTop: "6px" }}>
              <Btn small primary>install</Btn>
              <Btn small>details</Btn>
            </div>
          </div>
        </Box>
      ))}
    </div>
  </div>
);

// ---------- INSTALL ----------
const ImportPreviewTabs = ({ active, setActive, merge }) => {
  const tabs = ["Summary", "BGEE WeiDU", "BG2EE WeiDU", "User Downloads", "Installed Refs", "Mod Configs"];
  return (
    <div style={{
      display: "flex",
      gap: 4,
      flexWrap: "wrap",
      borderBottom: "1.5px solid var(--border-strong)",
      marginBottom: merge ? "-1.5px" : 14,
      position: merge ? "relative" : undefined,
      zIndex: merge ? 1 : undefined,
      flexShrink: 0,
    }}>
      {tabs.map((t) => {
        const isActive = active === t;
        return (
          <div
            key={t}
            onClick={() => setActive(t)}
            style={{
              padding: "6px 14px",
              cursor: "pointer",
              fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
              fontSize: 14,
              border: "1.5px solid var(--border-strong)",
              borderBottom: isActive ? "1.5px solid var(--shell-bg)" : "1.5px solid var(--border-strong)",
              borderRadius: "4px 4px 0 0",
              background: isActive ? "var(--shell-bg)" : "var(--chrome-bg)",
              color: isActive ? "var(--text)" : "var(--text-muted)",
              fontWeight: isActive ? 700 : 400,
              marginBottom: "-1.5px",
              userSelect: "none",
              whiteSpace: "nowrap",
            }}
          >
            {t}
          </div>
        );
      })}
    </div>
  );
};

const PreviewText = ({ tab }) => {
  const commonStyle = {
    fontFamily: "'FiraCode Nerd', monospace",
    fontSize: 13,
    lineHeight: 1.35,
    whiteSpace: "pre-wrap",
    color: "var(--text)",
  };
  const text = {
    "Summary": `BIO Modlist Import Preview\n\nModlist\nBIO version: 0.1.0-beta.19\nGame install: EET\nInstall mode: start_from_weidu_logs_then_review_edit\n\nWeiDU Logs\nBGEE: 21 entries\nBG2EE: 115 entries\n\nIncluded Data\nSource overrides: Yes\nInstalled refs / pins: Yes\nMod config files: 4\n\nWhat Import Will Do\n- Set game/install mode from this share code.\n- Write imported WeiDU logs.\n- Import source overrides if included.\n- Import installed refs/pins if included.\n- Store pending mod config files if included.\n- Keep local game, mods, archive, and backup paths unchanged.`,
    "BGEE WeiDU": `// Log of Currently Installed WeiDU Mods\n// The top of the file is the 'oldest' mod\n// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]\n~DLCMERGER\\DLCMERGER.TP2~ #0 #1 // Merge DLC into game -> Merge "Siege of Dragonspear" DLC: 1.8\n~EEFIXPACK\\SETUP-EEFIXPACK.TP2~ #0 #0 // Core Fixes: Beta 2\n~EEFIXPACK\\SETUP-EEFIXPACK.TP2~ #0 #2 // Game Text Update: Beta 2\n~BG1UB\\BG1UB.TP2~ #0 #10 // Place Entar Silvershield in His Home: v17.1\n~BG1UB\\BG1UB.TP2~ #0 #11 // Scar and the Sashenstar's Daughter: v17.1\n~BG1UB\\BG1UB.TP2~ #0 #12 // Quoningar, the Cleric: v17.1\n~BG1UB\\BG1UB.TP2~ #0 #13 // Shilo Chen and the Ogre-Magi: v17.1\n~BG1UB\\BG1UB.TP2~ #0 #14 // Edie, the Merchant League Applicant: v17.1`,
    "BG2EE WeiDU": `// Log of Currently Installed WeiDU Mods\n// The top of the file is the 'oldest' mod\n// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]\n~EEFIXPACK\\SETUP-EEFIXPACK.TP2~ #0 #0 // Core Fixes: Beta 2\n~EET\\EET.TP2~ #0 #0 // EET core (resource importation): v14.0 // @wlb-inputs: y,D:\\BGTesting\\test1\n~EEEX\\EEEX.TP2~ #0 #0 // EEex: v0.11.0-alpha\n~LEUI\\LEUI.TP2~ #0 #0 // Lefreut's Enhanced UI - Core component: 4.9.1\n~EEUITWEAKS\\EEUITWEAKS.TP2~ #0 #1030 // Portrait Selectors -> BillyYank's Multi-Portrait Mod: 4.0.7\n~STRATAGEMS\\SETUP-STRATAGEMS.TP2~ #0 #4210 // Randomize the maze in Watcher's Keep: 35.21\n~STRATAGEMS\\SETUP-STRATAGEMS.TP2~ #0 #5080 // Improved BG Textscreens: 35.21`,
    "User Downloads": `# BIO mod download user file\n\n[[mods]]\nname = "Tweaks Anthology"\ntp2 = "cdtweaks"\n\n  [[mods.sources]]\n  id = "gibberlings3"\n  label = "Gibberlings3"\n  type = "github"\n  url = "https://github.com/Gibberlings3/Tweaks-Anthology"\n  repo = "Gibberlings3/Tweaks-Anthology"\n  config_files = ["cdtweaks.ini", "desktop.ini", "gttweaks.ini"]\n\n[[mods]]\nname = "Sword Coast Stratagems"\ntp2 = "stratagems"`,
    "Installed Refs": `[refs]\ncdtweaks = "master@1069057e496e"\neefixpack = "Beta_2"\neet = "master@b164f5997de7"\neet_end = "master@b164f5997de7"\nstratagems = "v35.21"\n\n[sources]\nbg1aerie = "the-gate-project"\nbg1ub = "pocket-plane-group"\ncdtweaks = "gibberlings3"\ndlcmerger = "argent77"\neefixpack = "gibberlings3"\neet = "gibberlings3"\nstratagems = "gibberlings3"`,
    "Mod Configs": `cdtweaks | gibberlings3 | cdtweaks.ini\ncdtweaks | gibberlings3 | desktop.ini\ncdtweaks | gibberlings3 | gttweaks.ini\nstratagems | gibberlings3 | stratagems.ini`,
  };
  return <div style={commonStyle}>{text[tab]}</div>;
};

const InstallProgressScreen = ({ onBack }) => {
  const lines = [
    { kind: "info", text: "[install] starting mod_installer · 9 mods · 136 components" },
    { kind: "info", text: "[DLCMerger\\DLCMerger.TP2] Installing: Merge DLC into game -> Merge \"Siege of Dragonspear\" DLC: 1.8" },
    { kind: "green", text: "SUCCESSFULLY INSTALLED Merge \"Siege of Dragonspear\" DLC: 1.8" },
    { kind: "info", text: "[EEFIXPACK\\SETUP-EEFIXPACK.TP2] Installing: Core Fixes: Beta 2" },
    { kind: "info", text: "  patching .CRE files (1247) ..." },
    { kind: "info", text: "  patching .ITM files (823) ..." },
    { kind: "info", text: "  patching .SPL files (612) ..." },
    { kind: "green", text: "SUCCESSFULLY INSTALLED Core Fixes: Beta 2" },
    { kind: "info", text: "[EET\\EET.TP2] Installing: EET core (resource importation): v14.0" },
    { kind: "info", text: "  copying BG1 → BG2EE: 14,328 files ..." },
    { kind: "info", text: "  patching maps and dialogues ..." },
    { kind: "muted", text: "(streaming live...)" },
  ];
  return (
    <div className="sk-page">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end", marginBottom: 12 }}>
        <ScreenTitle title="Installing modlist" sub={`${FORK_META.name} · live install console`} />
        <Btn small onClick={onBack}>← back to import</Btn>
      </div>
      <Box style={{ padding: "10px 14px", marginBottom: 12, display: "flex", alignItems: "center", gap: 14 }}>
        <Pill tone="info">Installing</Pill>
        <Label>Component 18 / 136</Label>
        <Label hand style={{ color: "var(--text-faint)" }}>ran 1:23 · auto-answering prompts</Label>
        <Label hand style={{ marginLeft: "auto", color: "var(--text-faint)" }}>console auto-scrolls as new lines arrive</Label>
      </Box>
      <div style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center", flexWrap: "wrap" }}>
        <Btn primary>Cancel Install</Btn>
        <TopButton>Actions ▾</TopButton>
        <TopButton>Diagnostics ▾</TopButton>
        <TopButton>Prompt Answers</TopButton>
        <span style={{ width: 14 }} />
        <Label>● General</Label>
        <Label style={{ color: "var(--text-faint)" }}>○ Important Only</Label>
        <Label style={{ color: "var(--text-faint)" }}>○ Installed Only</Label>
        <Label>☑ Auto-scroll</Label>
      </div>
      <Box label="Console" style={{ padding: 10, minHeight: 360 }}>
        {lines.map((l, i) => (
          <Mono
            key={i}
            green={l.kind === "green"}
            muted={l.kind === "muted"}
            style={{ color: l.kind === "info" ? "var(--text)" : undefined }}
          >
            {l.text}
          </Mono>
        ))}
      </Box>
      <Box style={{ padding: "8px 12px", marginTop: 10, display: "flex", alignItems: "center", gap: 10 }}>
        <Label hand style={{ color: "var(--text-faint)", flexShrink: 0 }}>Type a prompt response:</Label>
        <div style={{ flex: 1, ...sketchyBorder, background: "var(--input-bg)", padding: "4px 8px", fontFamily: "'FiraCode Nerd', monospace", fontSize: 13, color: "var(--text-faint)" }}>
          (waiting for prompt to need input...)
        </div>
        <Btn small disabled>Send</Btn>
      </Box>
    </div>
  );
};

const InstallScreen = ({ reinstallTarget }) => {
  // stages: paste | preview | downloading | installing
  // When entering via Reinstall from Home, jump straight to "preview" with destination + share code pre-populated.
  const [stage, setStage] = useState(reinstallTarget ? "preview" : "paste");
  const [tab, setTab] = useState("Summary");
  const [dest, setDest] = useState(reinstallTarget ? reinstallTarget.destination : "");
  const [destChoice, setDestChoice] = useState(null);
  const isReinstall = !!reinstallTarget;
  const [forkInfoOpen, setForkInfoOpen] = useState(false);

  const isPartial = destChoice === "continue";
  const handleBrowse = () => {
    setDest("D:\\BG2EE_install_test");
    setDestChoice(null);
  };

  if (stage === "downloading") {
    return (
      <ImportDownloadScreen
        title="Downloading & extracting"
        sub="fetching mod archives — install starts automatically when ready"
        hint="after download: install runs without further prompts (no review step)"
        onCancel={() => setStage("preview")}
        onContinue={() => setStage("installing")}
        continueLabel="start install →"
      />
    );
  }

  if (stage === "installing") {
    return <InstallProgressScreen onBack={() => setStage("paste")} />;
  }

  if (stage === "paste") {
    return (
      <div className="sk-page" style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, padding: "20px 28px" }}>
        <ScreenTitle title="Install shared modlist" sub={isPartial ? "destination has existing modlist — share code skipped" : "set destination + mods paths, paste a BIO share code, then preview before importing"} />
        <Box style={{ padding: "16px 20px", marginBottom: 14, flexShrink: 0 }}>
          <FolderInput
            label="destination folder"
            placeholder="D:\BG2EE_install_test"
            value={dest}
            onBrowse={handleBrowse}
          />
          {dest && <DestinationNotEmptyWarning choice={destChoice} setChoice={setDestChoice} />}
        </Box>

        {isPartial ? (
          <Box style={{ padding: "20px", flexShrink: 0 }}>
            <Label style={{ fontWeight: 500, fontSize: 14, marginBottom: 4 }}>Continue partial installation</Label>
            <Label hand style={{ color: "var(--text-muted)", fontSize: 14 }}>
              Existing mod files detected at {dest}. Share-code entry is skipped — BIO will pick up where the previous install left off.
            </Label>
          </Box>
        ) : (
          <Box label="import code" style={{ padding: "20px", flexShrink: 0 }}>
            <Label style={{ marginBottom: "8px" }}>BIO-MODLIST-V1 share code</Label>
            <div style={{
              ...sketchyBorder,
              minHeight: 200,
              padding: "12px",
              background: "var(--input-bg)",
              fontFamily: "'FiraCode Nerd', monospace",
              fontSize: 12,
              color: "var(--text-faint)",
              whiteSpace: "pre-wrap",
            }}>
              BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...{"\n\n"}Paste the full code here.
            </div>
          </Box>
        )}

        <div style={{ flex: 1 }} />
        <SubFlowFooter
          onPrimary={() => setStage(isPartial ? "installing" : "preview")}
          primaryLabel={isPartial ? "Continue Install →" : "Preview →"}
          hint={isPartial ? "no share code needed" : "no install starts until preview is accepted"}
        />
      </div>
    );
  }

  // preview
  return (
    <div className="sk-page" style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, padding: "20px 28px" }}>
      <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12 }}>
        <ScreenTitle
          title={isReinstall ? `Reinstall "${reinstallTarget.n}"` : FORK_META.name}
          sub={isReinstall
            ? `overwrite install mode · existing files at ${dest} will be replaced before install runs`
            : `by ${FORK_META.author} · review what will be installed before BIO downloads anything`}
        />
        {!isReinstall && FORK_META.forkedFrom && FORK_META.forkedFrom.length > 0 && (
          <div style={{ flexShrink: 0, marginTop: 4 }}>
            <Btn small onClick={() => setForkInfoOpen(true)}>⑂ fork info</Btn>
          </div>
        )}
      </div>
      {isReinstall && (
        <Box style={{
          padding: "10px 14px",
          marginBottom: 12,
          background: "rgba(230,154,150,0.12)",
          flexShrink: 0,
        }}>
          <Label hand style={{ color: "#e69a96", fontSize: 13 }}>
            ⚠ Overwrite install · the existing install folder will be wiped before the install runs. prepare_target_dirs_before_install is forced ON.
          </Label>
        </Box>
      )}
      <Box style={{ padding: "14px", flexShrink: 0, marginBottom: 12 }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "6px 16px", fontSize: 14 }}>
          <Label>Game: <strong>{FORK_META.game}</strong></Label>
          <Label>Mods: <strong>{FORK_META.mods}</strong></Label>
          <Label>Components: <strong>{FORK_META.components}</strong></Label>
          <Label>BGEE/BG2EE entries: <strong>{FORK_META.bgeeEntries}/{FORK_META.bg2eeEntries}</strong></Label>
        </div>
      </Box>
      <ImportPreviewTabs active={tab} setActive={setTab} merge />
      <Box style={{ padding: "14px", flex: 1, minHeight: 0, overflow: "auto" }}>
        <PreviewText tab={tab} />
      </Box>
      <SubFlowFooter
        onBack={isReinstall ? null : () => setStage("paste")}
        onPrimary={() => setStage("downloading")}
        primaryLabel={isReinstall ? "Reinstall →" : "Import Modlist →"}
        hint={isReinstall ? "wipes target dirs, then runs install" : "downloads, extracts, then runs install — no review step"}
      />
      <ForkInfoPopup
        open={forkInfoOpen}
        onClose={() => setForkInfoOpen(false)}
        lineage={FORK_META.forkedFrom}
        self={{ name: FORK_META.name, author: FORK_META.author }}
      />
    </div>
  );
};

// ---------- CREATE / EDIT WORKSPACE ----------
const WorkspaceTabs = ({ active }) => {
  const tabs = [
    { id: "sources", label: "Sources", hint: "BIO Step 2" },
    { id: "components", label: "Components", hint: "BIO Step 3" },
    { id: "order", label: "Order", hint: "BIO Step 3" },
    { id: "final", label: "Final Plan", hint: "BIO Step 4" },
    { id: "install", label: "Install", hint: "BIO Step 5" },
  ];
  return (
    <div style={{ display: "flex", gap: 6, marginBottom: 14, borderBottom: "1.5px solid var(--border-strong)" }}>
      {tabs.map((t) => (
        <div key={t.id} style={{
          padding: "8px 14px",
          border: "1.5px solid var(--border-strong)",
          borderBottom: active === t.id ? "1.5px solid var(--shell-bg)" : "1.5px solid var(--border-strong)",
          background: active === t.id ? "var(--shell-bg)" : "var(--chrome-bg)",
          marginBottom: "-1.5px",
          borderRadius: "4px 4px 0 0",
          minWidth: 105,
        }}>
          <Label style={{ fontSize: 15, fontWeight: active === t.id ? 700 : 400 }}>{t.label}</Label>
          <Label hand style={{ fontSize: 12, color: "var(--text-faint)", lineHeight: 1 }}>{t.hint}</Label>
        </div>
      ))}
    </div>
  );
};

const Pill = ({ children, tone = "neutral", style, onClick, title }) => {
  // danger = soft coral, warn = amber, info = soft teal (harmonizes with --accent), neutral = warm grey
  const bg = tone === "danger" ? "#e69a96"
    : tone === "warn" ? "#e8c441"
    : tone === "info" ? "#a8d2cc"
    : "#c4cad1";
  return (
    <span
      className={onClick ? "sk-pill is-clickable" : "sk-pill"}
      onClick={onClick ? (e) => { e.stopPropagation(); onClick(e); } : undefined}
      title={title}
      style={{
        background: bg,
        color: "#1a2638",
        borderRadius: 7,
        padding: "1px 6px",
        fontSize: 10,
        fontWeight: 500,
        marginLeft: 8,
        whiteSpace: "nowrap",
        display: "inline-block",
        lineHeight: 1.45,
        cursor: onClick ? "pointer" : "default",
        ...style,
      }}
    >
      {children}
    </span>
  );
};

const Mono = ({ children, green, muted, style }) => (
  <div style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 14, lineHeight: 1.45, color: green ? "var(--success)" : muted ? "var(--text-faint)" : "var(--text)", ...style }}>{children}</div>
);

// Colors a WeiDU-style line: ~TP2\TP2.TP2~ (pink) #0 #ID (blue) // name: version (green)
const WeiduLine = ({ c, style }) => (
  <div style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 13, lineHeight: 1.45, whiteSpace: "nowrap", ...style }}>
    <span style={{ color: "#d4a35c" }}>~{c.tp2}\{c.tp2}.TP2~</span>
    <span> </span>
    <span style={{ color: "#2f6fb7" }}>#0 #{c.id}</span>
    <span style={{ color: "var(--success)" }}> // {c.name}: {c.version}</span>
  </div>
);

const TopButton = ({ children, primary, disabled, onClick }) => <Btn small primary={primary} disabled={disabled} onClick={onClick} style={{ fontSize: 13, padding: "5px 10px" }}>{children}</Btn>;

// Shared dataset used by Step 2 tree, Step 3 reorder list, and Step 4 review.
// Per-tab data — BGEE bucket and BG2EE bucket (for EET mode, both buckets are active).
const MOCK_MODS_BGEE = [
  { tp2: "DLCMERGER", name: "DLCMerger", version: "1.8", expanded: true, components: [
    { id: 1, name: 'Merge DLC into game -> Merge "Siege of Dragonspear" DLC', checked: true, order: 1 },
  ]},
  { tp2: "EEFIXPACK", name: "EEFixPack", version: "Beta 2", expanded: true, components: [
    { id: 0, name: "Core Fixes", checked: true, order: 2 },
    { id: 2, name: "Game Text Update", checked: true, order: 3 },
    { id: 5, name: "Drow Item Restorations", checked: false },
  ]},
  { tp2: "BG1UB", name: "BG1 Unfinished Business", version: "v17.1", expanded: true, components: [
    { id: 0, name: "Ice Island Level Two Restoration", checked: true, order: 4 },
    { id: 10, name: "Place Entar Silvershield in His Home", checked: true, order: 5 },
    { id: 11, name: "Scar and the Sashenstar's Daughter", checked: true, order: 6 },
    { id: 12, name: "Quoningar, the Cleric", checked: true, order: 7 },
    { id: 13, name: "Shilo Chen and the Ogre-Magi", checked: true, order: 8 },
    { id: 14, name: "Edie, the Merchant League Applicant", checked: true, order: 9 },
    { id: 16, name: "Creature Corrections", checked: true, order: 10 },
    { id: 17, name: "Creature Restorations", checked: true, order: 11 },
    { id: 19, name: "Minor Dialogue Restorations", checked: true, order: 12 },
  ]},
  { tp2: "BG1AERIE", name: "BG1Aerie", version: "2.5", expanded: false, components: [
    { id: 5000, name: "Aerie for BG:EE", checked: false },
    { id: 5001, name: "Install a lighter variant of the portrait for Aerie", checked: false },
  ]},
];

const MOCK_MODS_BG2EE = [
  { tp2: "EEFIXPACK", name: "EEFixPack", version: "Beta 2", expanded: true, components: [
    { id: 0, name: "Core Fixes", checked: true, order: 1 },
    { id: 2, name: "Game Text Update", checked: true, order: 2 },
    { id: 5, name: "Drow Item Restorations", checked: false },
  ]},
  { tp2: "EET", name: "EET", version: "v14.0", expanded: true, components: [
    { id: 0, name: "EET core (resource importation)", checked: true, order: 3, prompt: true },
  ]},
  { tp2: "EEEX", name: "EEex", version: "v0.11.0-alpha", expanded: true, components: [
    { id: 0, name: "EEex", checked: true, order: 4 },
  ]},
  { tp2: "EEUITWEAKS", name: "EEUITweaks", version: "4.0.7", expanded: true, components: [
    { id: 1000, name: "Mods Options", checked: true, order: 14 },
    { id: 1010, name: "Hidden Game Options", checked: true, order: 15 },
    { parent: true, name: "Portrait Selectors", subcomponents: [
      { id: 1030, name: "BillyYank's Multi-Portrait Mod", checked: true, order: 16 },
      { id: 1031, name: "Vanilla portraits only", checked: false },
      { id: 1032, name: "Custom portrait pack", checked: false },
    ]},
    { id: 1020, name: "Mr2150's Random PC Generator", checked: true, order: 17, conflict: true },
    { id: 1042, name: "Mr2150's Backup M_BG.lua", checked: false, conditional: true },
    { id: 1070, name: "Faydark's Abilities Auto-Roller", checked: true, order: 18, conflict: true },
    { id: 1100, name: "Display max proficiency limits", checked: true, order: 19 },
    { id: 2030, name: "Adul's Better Quick Loot", checked: true, order: 20 },
    { id: 2050, name: "Simple Centered Dialog", checked: true, order: 21 },
    { id: 2060, name: "lefreut's 1.3-ish Dialog Box", checked: true, order: 22, conflict: true },
  ]},
  { tp2: "LEUI", name: "LeUI", version: "4.9.1", expanded: false, components: [
    { id: 0, name: "lefreut's Enhanced UI - Core component", checked: true, order: 23 },
    { id: 1, name: "BG2 vanilla bams for spells", checked: true, order: 24 },
  ]},
  { tp2: "STRATAGEMS", name: "Sword Coast Stratagems", version: "35.21", expanded: true, components: [
    { id: 1500, name: "Initialise AI components", checked: true, order: 25 },
    { id: 4210, name: "Randomize the maze in Watcher's Keep", checked: true, order: 26 },
    { parent: true, name: "Improved Watcher's Keep Maze · difficulty", subcomponents: [
      { id: 4220, name: "Standard difficulty", checked: false },
      { id: 4221, name: "Hard difficulty", checked: true, order: 27, prompt: true },
      { id: 4222, name: "Insane difficulty", checked: false },
    ]},
    { id: 5080, name: "Improved BG Textscreens", checked: true, order: 28 },
    { id: 6000, name: "Smarter mages", checked: true, order: 29 },
    { id: 6010, name: "Smarter priests", checked: false },
  ]},
  { tp2: "CDTWEAKS", name: "Tweaks Anthology", version: "v17", expanded: false, components: [
    { id: 10, name: "Maximum HP for NPCs", checked: true, order: 30 },
    { id: 2090, name: "Multi-class grandmastery", checked: true, order: 31 },
    { id: 3010, name: "Quick-cast spells", checked: false },
  ]},
];

const MOCK_MODS_BY_TAB = { bgee: MOCK_MODS_BGEE, bg2ee: MOCK_MODS_BG2EE, iwdee: MOCK_MODS_BGEE };

// game ∈ {"EET","BGEE","BG2EE","IWDEE"} → list of tab descriptors. EET has both BGEE+BG2EE buckets;
// single-game modes show only their own tab in Steps 2–4.
const tabsForGame = (game) => {
  if (game === "EET") return [{ id: "bgee", label: "BGEE" }, { id: "bg2ee", label: "BG2EE" }];
  if (game === "BG2EE") return [{ id: "bg2ee", label: "BG2EE" }];
  if (game === "IWDEE") return [{ id: "iwdee", label: "IWDEE" }];
  return [{ id: "bgee", label: "BGEE" }];
};

// Flatten a mod's components tree (parent → subcomponents) into installable leaves.
const flattenComponents = (cs) =>
  cs.flatMap((c) => c.parent ? c.subcomponents.map((s) => ({ ...s, parentName: c.name })) : [c]);

const collectSelectedInOrder = (gameTab) => {
  const mods = MOCK_MODS_BY_TAB[gameTab] || [];
  return mods
    .flatMap((m) =>
      flattenComponents(m.components)
        .filter((c) => c.checked)
        .map((c) => ({ ...c, tp2: m.tp2, modName: m.name, version: m.version }))
    )
    .sort((a, b) => a.order - b.order);
};

const ComponentTree = ({ gameTab = "bgee", onOpenDetails, selectedKey, onSelect, onShowPopup, onShowCompat, onShowPrompt, orderByTab, setOrderByTab }) => {
  const select = (key, item) => onSelect && onSelect(key, item);
  const popup = (popupData) => onShowPopup && onShowPopup(popupData);
  const showCompat = (args) => onShowCompat && onShowCompat(args);
  const showPrompt = (args) => onShowPrompt && onShowPrompt(args);
  // Expanded state: a Set of keys ("tab:m:TP2" for mods, "tab:p:TP2:NAME" for parent components).
  // Initialized from MOCK data so existing expanded=true entries open by default; parent components default to expanded.
  const [expanded, setExpanded] = useState(() => {
    const s = new Set();
    for (const gt of Object.keys(MOCK_MODS_BY_TAB)) {
      for (const m of MOCK_MODS_BY_TAB[gt] || []) {
        if (m.expanded) s.add(`${gt}:m:${m.tp2}`);
        for (const c of m.components) {
          if (c.parent) s.add(`${gt}:p:${m.tp2}:${c.name}`);
        }
      }
    }
    return s;
  });
  const isExpanded = (key) => expanded.has(key);
  const toggleExpanded = (key) => setExpanded((s) => {
    const next = new Set(s);
    if (next.has(key)) next.delete(key); else next.add(key);
    return next;
  });
  // Checked state derives from the lifted orderByTab: a component is "checked" iff it appears
  // in the active tab's ordered array. Toggling on appends to end; toggling off removes by key.
  const checkedKeys = React.useMemo(() => {
    const s = new Set();
    for (const c of (orderByTab && orderByTab[gameTab]) || []) s.add(`${gameTab}:${c.tp2}:${c.id}`);
    return s;
  }, [orderByTab, gameTab]);
  const isChecked = (key) => checkedKeys.has(key);
  const toggleCheckedComponent = (m, c) => {
    const itemKey = `${m.tp2}:${c.id}`;
    setOrderByTab((prev) => {
      const cur = prev[gameTab] || [];
      const idx = cur.findIndex((it) => `${it.tp2}:${it.id}` === itemKey);
      if (idx >= 0) {
        const next = [...cur.slice(0, idx), ...cur.slice(idx + 1)];
        return { ...prev, [gameTab]: next };
      }
      // Append with the same enriched shape collectSelectedInOrder produces
      const newItem = { ...c, tp2: m.tp2, modName: m.name, version: m.version };
      return { ...prev, [gameTab]: [...cur, newItem] };
    });
  };
  // Batch toggle: when target=true add missing leaves to end; when false remove all matching.
  const setBulkChecked = (m, leaves, target) => {
    setOrderByTab((prev) => {
      const cur = prev[gameTab] || [];
      const toggleKeys = new Set(leaves.map((c) => `${m.tp2}:${c.id}`));
      if (!target) {
        return { ...prev, [gameTab]: cur.filter((c) => !toggleKeys.has(`${c.tp2}:${c.id}`)) };
      }
      const existing = new Set(cur.map((c) => `${c.tp2}:${c.id}`));
      const toAdd = leaves
        .filter((c) => !existing.has(`${m.tp2}:${c.id}`))
        .map((c) => ({ ...c, tp2: m.tp2, modName: m.name, version: m.version }));
      return { ...prev, [gameTab]: [...cur, ...toAdd] };
    });
  };
  // Radio toggle: when a subcomponent under a WeiDU subcomponent-parent is checked,
  // uncheck all of its siblings (only one can be installed). Clicking an already-checked
  // subcomponent unchecks it (leaves the group with no variant chosen).
  const toggleRadioInGroup = (m, c, parentGroup) => {
    setOrderByTab((prev) => {
      let cur = prev[gameTab] || [];
      const itemKey = `${m.tp2}:${c.id}`;
      const wasChecked = cur.some((it) => `${it.tp2}:${it.id}` === itemKey);
      const siblingKeys = new Set(parentGroup.subcomponents.map((s) => `${m.tp2}:${s.id}`));
      cur = cur.filter((it) => !siblingKeys.has(`${it.tp2}:${it.id}`));
      if (!wasChecked) {
        cur = [...cur, { ...c, tp2: m.tp2, modName: m.name, version: m.version }];
      }
      return { ...prev, [gameTab]: cur };
    });
  };
  // renderLeaf: parentGroup, when present, enables radio (pick-one) behavior on the
  // subcomponent's checkbox. The row itself is selectable either way (BIO selects the
  // component on row click without toggling its checkbox).
  const renderLeaf = (c, m, indent, key, parentGroup = null) => {
    const checkKey = `${gameTab}:${m.tp2}:${c.id}`;
    const checked = isChecked(checkKey);
    const isSubcomponent = !!parentGroup;
    const onToggle = (e) => {
      e.stopPropagation();
      if (isSubcomponent) {
        toggleRadioInGroup(m, c, parentGroup);
      } else {
        toggleCheckedComponent(m, c);
      }
      select(key, { ...c, kind: "leaf", modName: m.name, tp2: m.tp2, version: m.version });
    };
    return (
    <div
      key={key}
      className={`tree-row${selectedKey === key ? " is-selected" : ""}`}
      onClick={() => select(key, { ...c, kind: "leaf", modName: m.name, tp2: m.tp2, version: m.version })}
      style={{ paddingLeft: indent, paddingRight: 4, display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}
    >
      <span
        onClick={onToggle}
        title={isSubcomponent ? (checked ? "Uncheck (no variant chosen)" : "Pick this variant") : (checked ? "Uncheck component" : "Check component")}
        style={{
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          width: 14,
          height: 14,
          border: "1px solid var(--text-faint)",
          borderRadius: isSubcomponent ? "50%" : 2,
          flexShrink: 0,
          cursor: "pointer",
          boxSizing: "border-box",
          userSelect: "none",
        }}
      >
        {checked && isSubcomponent && (
          <span style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--text)" }} />
        )}
        {checked && !isSubcomponent && (
          <span style={{ color: "var(--text)", fontSize: 11, lineHeight: 1, fontWeight: 700 }}>✓</span>
        )}
      </span>
      {checked && (
        <span style={{ color: "var(--text-faint)", fontSize: 10, minWidth: 24, flexShrink: 0, whiteSpace: "nowrap" }}>
          #{String(c.order || "—").toString().padStart(2, "0")}
        </span>
      )}
      <span style={{ color: checked ? "#2f6fb7" : "var(--text-faint)", whiteSpace: "nowrap", flexShrink: 0 }}>#0 #{c.id}</span>
      <span style={{
        color: checked ? "var(--success)" : "var(--text-faint)",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        minWidth: 0,
        flexShrink: 1,
      }}>
        // {c.name}: {m.version}
      </span>
      {c.conflict && (
        <Pill
          tone="danger"
          title="Compatibility issue — click for details"
          onClick={() => {
            select(key, { ...c, kind: "leaf", modName: m.name, tp2: m.tp2, version: m.version });
            showCompat({ mode: "single", initialFilter: "Conflict", initialItem: { tp2: m.tp2, id: c.id, label: c.name, kind: "conflict", summary: "Conflicts with another component installed earlier in the order", reason: "This component edits the same resource files as a previously-installed component. The two edits are not stackable, so the later one will overwrite the earlier one.", source: "step2_compat_rules_default.toml" } });
          }}
        >Conflict</Pill>
      )}
      {c.conditional && (
        <Pill
          tone="info"
          title="Conditional install — click for details"
          onClick={() => {
            select(key, { ...c, kind: "leaf", modName: m.name, tp2: m.tp2, version: m.version });
            showCompat({ mode: "single", initialFilter: "Conditional", initialItem: { tp2: m.tp2, id: c.id, label: c.name, kind: "conditional", summary: "Installs only when its dependency is selected", reason: "Component will install only if its dependencies are present. Detected: requires Game Text Update (#2). If unmet, this component is silently skipped during install.", source: "step2_compat_rules_default.toml", relatedMod: m.tp2, relatedComponent: 2 } });
          }}
        >Conditional</Pill>
      )}
      {c.prompt && (
        <Pill
          tone="warn"
          title="Parsed prompts — click for details"
          onClick={() => {
            select(key, { ...c, kind: "leaf", modName: m.name, tp2: m.tp2, version: m.version });
            showPrompt({ mode: "single", component: c, mod: m });
          }}
        >Prompt</Pill>
      )}
      <span
        className="tree-action"
        onClick={(e) => { e.stopPropagation(); select(key, { ...c, kind: "leaf", modName: m.name, tp2: m.tp2, version: m.version }); onOpenDetails && onOpenDetails(c); }}
        title="Show details"
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          color: "var(--text-muted)",
          fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
          fontSize: 11,
          fontWeight: 500,
          padding: "0 6px",
          lineHeight: "16px",
          cursor: "pointer",
          flexShrink: 0,
          userSelect: "none",
          marginLeft: "auto",
        }}
      >?</span>
    </div>
    );
  };
  return (
    <div style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 12, lineHeight: 1.5 }}>
      {(MOCK_MODS_BY_TAB[gameTab] || []).map((m) => {
        const leaves = flattenComponents(m.components);
        const total = leaves.length;
        const checkedCount = leaves.filter((c) => isChecked(`${gameTab}:${m.tp2}:${c.id}`)).length;
        const allChecked = checkedCount === total;
        const someChecked = checkedCount > 0 && checkedCount < total;
        const modExpKey = `${gameTab}:m:${m.tp2}`;
        const modExpanded = isExpanded(modExpKey);
        const expandGlyph = modExpanded ? "▾" : "▸";
        const conflictCount = leaves.filter((c) => c.conflict).length;
        const modKey = `m:${m.tp2}`;
        return (
          <React.Fragment key={m.tp2}>
            <div
              className={`tree-row${selectedKey === modKey ? " is-selected" : ""}`}
              onClick={() => { toggleExpanded(modExpKey); select(modKey, { ...m, kind: "mod" }); }}
              style={{ marginTop: 6, paddingRight: 6, fontWeight: 500, display: "flex", alignItems: "center", gap: 6, whiteSpace: "nowrap", cursor: "pointer" }}
            >
              <span
                style={{ userSelect: "none", fontSize: 16, padding: "2px 4px", margin: "-2px -4px" }}
              >{expandGlyph}</span>
              <span
                onClick={(e) => { e.stopPropagation(); setBulkChecked(m, leaves, !allChecked); }}
                title={allChecked ? "Uncheck all components" : "Check all components"}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                  width: 14,
                  height: 14,
                  border: "1px solid var(--text-faint)",
                  borderRadius: 2,
                  flexShrink: 0,
                  cursor: "pointer",
                  boxSizing: "border-box",
                  userSelect: "none",
                }}
              >
                {allChecked && (
                  <span style={{ color: "var(--text)", fontSize: 11, lineHeight: 1, fontWeight: 700 }}>✓</span>
                )}
                {someChecked && (
                  <span style={{ width: 7, height: 2, background: "var(--text)" }} />
                )}
              </span>
              <span>{m.name}</span>
              <span style={{ color: "var(--text-faint)", fontWeight: 400 }}>({checkedCount}/{total})</span>
              <span style={{ color: "var(--text-faint)", fontWeight: 400, fontSize: 11 }}>v{m.version}</span>
              {conflictCount > 0 && (
                <Pill
                  tone="danger"
                  title={`${conflictCount} compatibility issue${conflictCount > 1 ? "s" : ""} — click for details`}
                  onClick={() => {
                    select(modKey, { ...m, kind: "mod" });
                    const firstConflict = leaves.find((c) => c.conflict);
                    showCompat({
                      mode: "aggregate",
                      initialFilter: "Conflict",
                      initialItem: firstConflict ? { tp2: m.tp2, id: firstConflict.id, label: firstConflict.name, kind: "conflict" } : null,
                    });
                  }}
                >{conflictCount} conflict{conflictCount > 1 ? "s" : ""}</Pill>
              )}
              <span
                className="tree-action"
                onClick={(e) => { e.stopPropagation(); select(modKey, { ...m, kind: "mod" }); onOpenDetails && onOpenDetails(m); }}
                title="Show details"
                style={{
                  ...sketchyBorder,
                  background: "var(--shell-bg)",
                  color: "var(--text-muted)",
                  fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                  fontSize: 11,
                  fontWeight: 500,
                  padding: "0 6px",
                  lineHeight: "16px",
                  cursor: "pointer",
                  flexShrink: 0,
                  userSelect: "none",
                  marginLeft: "auto",
                }}
              >?</span>
            </div>
            {modExpanded && m.components.map((c) => {
              if (c.parent) {
                // WeiDU subcomponent-parent group (e.g., "Portrait Selectors"): purely a
                // collapsing header in BIO. NOT selectable. NO checkbox glyph (the group
                // itself isn't installable; only its subcomponents are, and they behave as
                // radio buttons — pick one). Clicking the row toggles expand/collapse only.
                const subChecked = c.subcomponents.filter((s) => isChecked(`${gameTab}:${m.tp2}:${s.id}`)).length;
                const subTotal = c.subcomponents.length;
                const parentExpKey = `${gameTab}:p:${m.tp2}:${c.name}`;
                const parentExpanded = isExpanded(parentExpKey);
                return (
                  <React.Fragment key={`parent-${c.name}`}>
                    <div
                      onClick={() => toggleExpanded(parentExpKey)}
                      style={{
                        paddingLeft: 30,
                        paddingRight: 6,
                        marginTop: 2,
                        display: "flex",
                        alignItems: "center",
                        gap: 6,
                        whiteSpace: "nowrap",
                        fontWeight: 500,
                        color: "var(--text-muted)",
                        cursor: "pointer",
                      }}
                    >
                      <span
                        style={{ userSelect: "none", fontSize: 14, lineHeight: 1, color: "var(--text-faint)", padding: "4px 6px", margin: "-4px -6px" }}
                      >{parentExpanded ? "▾" : "▸"}</span>
                      <span style={{ fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif", fontSize: 12 }}>{c.name}</span>
                      <span style={{ color: "var(--text-faint)", fontWeight: 400, fontSize: 11 }}>(pick one · {subChecked}/{subTotal})</span>
                    </div>
                    {parentExpanded && c.subcomponents.map((s) => renderLeaf(s, m, 46, `l:${m.tp2}:${s.id}`, c))}
                  </React.Fragment>
                );
              }
              return renderLeaf(c, m, 30, `l:${m.tp2}:${c.id}`);
            })}
          </React.Fragment>
        );
      })}
    </div>
  );
};

// Inline SVG icons (heroicons-style) — render in any font context, no Nerd dependency.
const ICON_COPY = (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ display: "block" }}>
    <rect x="9" y="9" width="13" height="13" rx="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </svg>
);
const ICON_OPEN = (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ display: "block" }}>
    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    <polyline points="15 3 21 3 21 9" />
    <line x1="10" y1="14" x2="21" y2="3" />
  </svg>
);

const DETAILS_GRID_ROW = {
  display: "grid",
  gridTemplateColumns: "120px minmax(0,1fr)",
  gap: 8,
  alignItems: "center",
  padding: "2px 0",
  position: "relative",
};

const RowIconBtn = ({ icon, title, onClick }) => (
  <Btn
    small
    title={title}
    onClick={onClick}
    style={{
      padding: "2px 6px",
      fontSize: 13,
      lineHeight: 1,
      minWidth: 24,
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
    }}
  >
    {icon}
  </Btn>
);

const DetailsRow = ({ label, value, actions = ["copy"], valueColor, copyable = true }) => {
  const [copied, setCopied] = useState(false);
  const valueIsString = typeof value === "string";
  const onCopy = () => {
    if (!copyable) return;
    if (typeof navigator !== "undefined" && navigator.clipboard && valueIsString) {
      navigator.clipboard.writeText(value).catch(() => {});
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  const hasActions = actions && actions.length > 0;
  return (
    <div className="details-row" style={DETAILS_GRID_ROW}>
      <Label style={{ minWidth: 0 }}>{label}</Label>
      <Label
        onClick={copyable ? onCopy : undefined}
        title={valueIsString ? value : undefined}
        style={{
          minWidth: 0,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          paddingRight: 4,
          cursor: copyable ? "pointer" : "default",
          ...(valueColor ? { color: valueColor } : null),
        }}
      >
        {value}
      </Label>
      <div
        className={`details-action${copied ? " is-copied" : ""}`}
        style={{
          position: "absolute",
          right: 0,
          top: 0,
          bottom: 0,
          display: "flex",
          alignItems: "center",
          gap: 4,
          paddingLeft: 8,
          background: "var(--shell-bg)",
        }}
      >
        {copied ? (
          <span
            style={{
              fontSize: 11,
              color: "var(--success)",
              fontWeight: 500,
              letterSpacing: 0.5,
            }}
          >
            Copied!
          </span>
        ) : hasActions ? (
          <>
            {actions.includes("copy") && (
              <RowIconBtn icon={ICON_COPY} title="Copy" onClick={onCopy} />
            )}
            {actions.includes("open") && (
              <RowIconBtn icon={ICON_OPEN} title="Open" />
            )}
          </>
        ) : null}
      </div>
    </div>
  );
};

// Collapsible block for Component Block / WeiDU Line sub-sections in leaf details.
const DetailsCollapse = ({ label, children, defaultOpen = true }) => {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div style={{ marginTop: 10 }}>
      <div
        onClick={() => setOpen((v) => !v)}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          cursor: "pointer",
          userSelect: "none",
          padding: "2px 0",
        }}
      >
        <span style={{ color: "var(--text-faint)", fontSize: 11 }}>{open ? "▾" : "▸"}</span>
        <Label hand style={{ flex: 1 }}>{label}</Label>
        <span
          className="details-action"
          onClick={(e) => {
            e.stopPropagation();
            // Wireframe: copy stub. In production, copy block contents.
            if (navigator.clipboard && navigator.clipboard.writeText) {
              navigator.clipboard.writeText(typeof children === "string" ? children : "").catch(() => {});
            }
          }}
          title="Copy"
          style={{ color: "var(--text-muted)", display: "flex", alignItems: "center", cursor: "pointer" }}
        >
          {ICON_COPY}
        </span>
      </div>
      {open && (
        <div style={{
          ...sketchyBorder,
          padding: 8,
          marginTop: 4,
          background: "var(--input-bg)",
          fontFamily: "'FiraCode Nerd', monospace",
          fontSize: 11,
          color: "var(--text)",
          whiteSpace: "pre-wrap",
          maxHeight: 220,
          overflow: "auto",
        }}>
          {children}
        </div>
      )}
    </div>
  );
};

const COMPONENT_BLOCK_SAMPLE = `BEGIN ~Mods Options~
  REQUIRE_PREDICATE GAME_IS ~bg2ee~ ~Not compatible with this game~
  DEFINE_ACTION_MACRO ~install_mods_options~ BEGIN
    COPY ~EEUITWEAKS/options/m_opt.lua~ ~override/m_opt.lua~
    APPEND_OUTER ~m_options.lua~ ~_init_opts()~ UNLESS ~_init_opts~
  END
LAF install_mods_options END`;

const WEIDU_LINE_SAMPLE = `~EEUITWEAKS/EEUITWEAKS.TP2~ #0 #1000 // Mods Options: 4.0.7`;

const DetailsPanel = ({ selected, style, onClose }) => {
  const closeBtn = onClose && (
    <span
      onClick={onClose}
      title="Close details"
      style={{
        position: "absolute",
        top: 6,
        right: 8,
        width: 22,
        height: 22,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
        fontSize: 16,
        lineHeight: 1,
        color: "var(--text-muted)",
        cursor: "pointer",
        userSelect: "none",
        borderRadius: 3,
      }}
    >✕</span>
  );

  // No selection → empty state.
  if (!selected) {
    return (
      <Box label="Details" style={{
        padding: 12, minHeight: 400, overflowX: "hidden", overflowWrap: "anywhere", ...style,
      }}>
        {closeBtn}
        <Label style={{ color: "var(--text-faint)", marginTop: 8 }}>
          Click a mod or component to see its details.
        </Label>
      </Box>
    );
  }

  // BIO has only Mod- and Component-level selection. Wireframe-only "parent" rows
  // (e.g. Portrait Selectors) fall back to the mod-header view since BIO has no parent-group details.
  const kind = selected.kind;
  const isMod = kind === "mod" || kind === "parent";
  const modName = isMod ? (selected.modName || selected.name) : selected.modName;
  const modTp2 = selected.tp2;
  const modVersion = selected.version || "Unknown";

  // Shared paths + package + action buttons (same shape for both kinds).
  const pathsLinks = (
    <>
      <Label hand>Paths / Links</Label>
      <div style={{ marginTop: 6 }}>
        <DetailsRow
          label="TP2 Folder"
          value={`D:\\import test\\Mods\\${modTp2}`}
          actions={["copy", "open"]}
        />
        <DetailsRow
          label="TP2 Path"
          value={`D:\\import test\\Mods\\${modTp2}\\${modTp2}.tp2`}
          actions={["copy", "open"]}
        />
        <DetailsRow
          label="INI Path"
          value={`D:\\import test\\Mods\\${modTp2}\\${modTp2}.ini`}
          actions={["copy", "open"]}
        />
        <DetailsRow label="Readme" value="No data" valueColor="#c9a93f" actions={[]} copyable={false} />
      </div>
    </>
  );

  const packageBlock = (
    <>
      <Label hand style={{ marginTop: 12 }}>Package</Label>
      <div style={{ marginTop: 6 }}>
        <DetailsRow label="Installed Source" value="gibberlings3" />
        <DetailsRow label="Update Source" value="gibberlings3 (default)" />
        <DetailsRow label="Latest Version" value={modVersion} />
        <DetailsRow
          label="URL"
          value={`https://github.com/Gibberlings3/${modTp2}`}
          actions={["copy", "open"]}
        />
        <DetailsRow
          label="GitHub"
          value={`Gibberlings3/${modTp2}`}
          actions={["copy", "open"]}
        />
      </div>
      <div style={{ marginTop: 12, display: "flex", gap: 8, flexWrap: "wrap" }}>
        <Btn small>Check This Mod</Btn>
        <Btn small>Lock Updates</Btn>
      </div>
    </>
  );

  if (isMod) {
    // Mod-header view (matches BIO's Step2Selection::Mod)
    return (
      <Box label="Details" style={{
        padding: 12, minHeight: 400, overflowX: "hidden", overflowWrap: "anywhere", ...style,
      }}>
        {closeBtn}
        <Label style={{ fontSize: 15, fontWeight: 500 }}>{modName}</Label>
        <Label hand style={{ fontSize: 12, color: "var(--text-faint)" }}>
          {kind === "parent" ? `Subcomponent group · ${modTp2}` : `Mod · ${modTp2}`}
        </Label>
        <Label style={{ marginTop: 6 }}>Version: {modVersion}</Label>
        <Label hand style={{ marginTop: 10 }}>Selection</Label>
        <div style={{ marginTop: 6 }}>
          <DetailsRow label="TP2 File" value={`${modTp2}.tp2`} />
          <DetailsRow label="Shown" value="62" />
          <DetailsRow label="Hidden" value="5" />
          <DetailsRow label="Raw" value="67" />
        </div>
        <div style={{ borderTop: "1.5px solid var(--border-soft)", margin: "12px 0" }} />
        {pathsLinks}
        {packageBlock}
      </Box>
    );
  }

  // Leaf-component view (matches BIO's Step2Selection::Component)
  const c = selected;
  const hasCompat = !!c.conflict || !!c.conditional;
  return (
    <Box label="Details" style={{
      padding: 12, minHeight: 400, overflowX: "hidden", overflowWrap: "anywhere", ...style,
    }}>
      {closeBtn}
      <Label style={{ fontSize: 15, fontWeight: 500 }}>{modName}</Label>
      <Label hand style={{ fontSize: 12, color: "var(--text-faint)" }}>
        Component #{c.id} · {modName}
      </Label>
      <Label style={{ marginTop: 6 }}>Version: {modVersion}</Label>
      <Label hand style={{ marginTop: 10 }}>Selection</Label>
      <div style={{ marginTop: 6 }}>
        <DetailsRow label="Component" value={`${c.name}: ${modVersion}`} />
        <DetailsRow label="ID" value={String(c.id)} />
        <DetailsRow
          label="Checked"
          value={c.checked ? "Checked" : "Unchecked"}
          valueColor={c.checked ? "var(--success)" : "var(--text-faint)"}
          actions={[]}
          copyable={false}
        />
        <DetailsRow
          label="State"
          value={c.conflict ? "Disabled" : "Selectable"}
          valueColor={c.conflict ? "#e8c441" : "var(--success)"}
          actions={[]}
          copyable={false}
        />
        <DetailsRow label="Language" value="en_US" />
        <DetailsRow label="TP2 File" value={`${modTp2}.tp2`} />
        <DetailsRow label="Shown" value="62" />
        <DetailsRow label="Hidden" value="5" />
        <DetailsRow label="Raw" value="67" />
        {c.checked && c.order && <DetailsRow label="Order" value={String(c.order)} />}
      </div>

      {hasCompat && (
        <>
          <div style={{ borderTop: "1.5px solid var(--border-soft)", margin: "12px 0" }} />
          <Label hand style={{ color: c.conflict ? "#e69a96" : "#a8d2cc" }}>
            Compatibility
          </Label>
          <div style={{ marginTop: 6 }}>
            <DetailsRow label="Source Type" value={c.conflict ? "Conflicts" : "Conditional"} actions={[]} copyable={false} />
            <DetailsRow label="Issue" value={c.conflict ? "FORBID_HIT" : "CONDITIONAL"} />
            <DetailsRow
              label="Reason"
              value={c.conflict
                ? `Edits the same resource files as a previously-installed component. The two edits are not stackable.`
                : `Installs only when its dependency is selected and conditions are met.`}
              valueColor="#e8c441"
              actions={[]}
              copyable={false}
            />
            <DetailsRow label="Rule Origin" value="step2_compat_rules_default.toml" />
            <DetailsRow label="Related" value={`${modTp2} #${(c.id || 0) - 1}`} />
            <DetailsRow label="Matched Rule" value={`mod = "${(modTp2 || "").toLowerCase()}", component_id = "${c.id}"`} />
          </div>
        </>
      )}

      <div style={{ borderTop: "1.5px solid var(--border-soft)", margin: "12px 0" }} />
      {pathsLinks}
      {packageBlock}
      <div style={{ borderTop: "1.5px solid var(--border-soft)", margin: "12px 0" }} />
      <DetailsCollapse label="Component Block">{COMPONENT_BLOCK_SAMPLE}</DetailsCollapse>
      <DetailsCollapse label="WeiDU Line" defaultOpen={false}>{WEIDU_LINE_SAMPLE}</DetailsCollapse>
    </Box>
  );
};

// --- Reusable header bits shared by Sources / Components / Order panels ---
const GameTab = ({ active, onClick, children }) => (
  <div
    onClick={onClick}
    style={{
      // Tab is taller than the action row so its bottom edge reaches the pane
      // below. The active tab's bottom border is shell-bg colored AND the tab
      // pulls down by -1.5px so it overlaps (and masks) the pane's top border —
      // visually the active tab "flows into" the pane below.
      padding: "5px 14px 8px 14px",
      display: "flex",
      alignItems: "center",
      boxSizing: "border-box",
      cursor: "pointer",
      fontSize: 13,
      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
      border: "1.5px solid var(--border-strong)",
      borderBottom: active ? "1.5px solid var(--shell-bg)" : "1.5px solid var(--border-strong)",
      borderRadius: "4px 4px 0 0",
      background: active ? "var(--shell-bg)" : "var(--chrome-bg)",
      color: active ? "var(--text)" : "var(--text-muted)",
      fontWeight: active ? 500 : 400,
      marginBottom: "-1.5px",
      userSelect: "none",
      whiteSpace: "nowrap",
    }}
  >
    {children}
  </div>
);

const Kebab = ({ items }) => {
  const [open, setOpen] = useState(false);
  React.useEffect(() => {
    if (!open) return undefined;
    const close = () => setOpen(false);
    const t = setTimeout(() => document.addEventListener("click", close, { once: true }), 0);
    return () => { clearTimeout(t); document.removeEventListener("click", close); };
  }, [open]);
  return (
    <div style={{ position: "relative" }}>
      <Btn small onClick={() => setOpen((v) => !v)} title="More actions" style={{ padding: "3px 9px", fontSize: 15, lineHeight: 1 }}>···</Btn>
      {open && (
        <div style={{
          position: "absolute",
          top: "calc(100% + 4px)",
          right: 0,
          minWidth: 180,
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "3px 3px 0 var(--shadow)",
          padding: 4,
          zIndex: 5,
        }}>
          {items.map((it, i) => (
            <div
              key={i}
              onClick={() => { setOpen(false); it.onClick && it.onClick(); }}
              style={{
                padding: "6px 10px",
                fontSize: 13,
                cursor: "pointer",
                borderRadius: 3,
                color: "var(--text)",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "var(--hover-overlay)")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
            >
              {it.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

// Top-left chevron used on every popup header. Click toggles `collapsed`.
// When collapsed, the popup body should be hidden so only the header bar is visible.
const PopupCollapseChevron = ({ collapsed, onClick }) => (
  <button
    onClick={(e) => { e.stopPropagation(); onClick && onClick(); }}
    title={collapsed ? "Expand" : "Collapse"}
    style={{
      all: "unset",
      cursor: "pointer",
      width: 18,
      height: 18,
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      fontSize: 11,
      color: "var(--text-muted)",
      marginRight: 8,
      flexShrink: 0,
      transform: collapsed ? "rotate(-90deg)" : "rotate(0deg)",
      transition: "transform 0.12s ease-out",
    }}
  >
    ▾
  </button>
);

// Captures the popup's top position when it collapses, so the chevron stays
// under the mouse instead of the popup re-centering to the middle of the screen.
// Returns: [collapsed, toggle, popupRef, anchorStyle]. Spread anchorStyle into
// the popup's inner div style. Attach popupRef as `ref` on the same div.
const usePopupCollapse = (open) => {
  const [collapsed, setCollapsed] = useState(false);
  const [pinnedTop, setPinnedTop] = useState(null);
  const ref = React.useRef(null);
  React.useEffect(() => { if (!open) { setCollapsed(false); setPinnedTop(null); } }, [open]);
  const toggle = () => {
    if (!collapsed && ref.current) {
      const rect = ref.current.getBoundingClientRect();
      setPinnedTop(rect.top);
    } else {
      setPinnedTop(null);
    }
    setCollapsed((c) => !c);
  };
  const anchorStyle = pinnedTop !== null
    ? { alignSelf: "flex-start", marginTop: `${pinnedTop}px` }
    : null;
  return [collapsed, toggle, ref, anchorStyle];
};

const ConfirmDialog = ({ open, title, message, confirmLabel = "Confirm", onConfirm, onCancel, danger }) => {
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  if (!open) return null;
  return (
    <div
      onClick={onCancel}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "5px 5px 0 var(--shadow)",
          padding: 18,
          maxWidth: 460,
          width: "92%",
          ...anchorStyle,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 8 }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 15, fontWeight: 500 }}>{title}</Label>
        </div>
        {!collapsed && <>
        <Label style={{ color: "var(--text-muted)", marginBottom: 16, fontSize: 13, lineHeight: 1.45 }}>{message}</Label>
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <Btn small onClick={onCancel}>Cancel</Btn>
          <Btn
            small
            primary
            onClick={onConfirm}
            style={danger ? { background: "#e69a96" } : undefined}
          >
            {confirmLabel}
          </Btn>
        </div>
        </>}
      </div>
    </div>
  );
};

const SAMPLE_PASTE_CODE = "BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQAlBQQ5MgAAYWxsLW1vZGxpc3Qtc2hhcmUtY29kZS1zYW1wbGUtZGF0YS1jb21wcmVzc2VkLWFuZC1lbmNvZGVkLWluLWJhc2U2NA==";

const SharePasteCodeDialog = ({ open, onClose, code = SAMPLE_PASTE_CODE, title = "Share import code", sub = "Anyone can paste this into BIO → Install to get the same modlist." }) => {
  const [copied, setCopied] = useState(false);
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  React.useEffect(() => { if (open) setCopied(false); }, [open]);
  if (!open) return null;
  const handleCopy = () => {
    if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(code).catch(() => {});
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <div
      onClick={onClose}
      style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{ ...sketchyBorder, background: "var(--shell-bg)", boxShadow: "5px 5px 0 var(--shadow)", padding: 20, maxWidth: 600, width: "94%", ...anchorStyle }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 6 }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 18, fontWeight: 500 }}>{title}</Label>
        </div>
        {!collapsed && <>
        <Label style={{ color: "var(--text-muted)", marginBottom: 14, fontSize: 13, lineHeight: 1.5 }}>{sub}</Label>
        <div style={{
          ...sketchyBorder,
          padding: 12,
          background: "var(--input-bg)",
          fontFamily: "'FiraCode Nerd', monospace",
          fontSize: 12,
          color: "var(--text)",
          marginBottom: 14,
          maxHeight: 180,
          overflow: "auto",
          wordBreak: "break-all",
          whiteSpace: "pre-wrap",
        }}>{code}</div>
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", alignItems: "center" }}>
          {copied && <Label hand style={{ color: "var(--success)", marginRight: "auto", fontSize: 14 }}>✓ copied to clipboard</Label>}
          <Btn small onClick={onClose}>Close</Btn>
          <Btn small primary onClick={handleCopy}>Copy</Btn>
        </div>
        </>}
      </div>
    </div>
  );
};

// Compatibility / conflict popup — matches BIO's compat window (compat_window_step2.rs).
// One window serves both single-issue and aggregate (toolbar / mod-row) entry points.
// All categories the rule engine supports. Tone drives the pill / status color.
const COMPAT_KIND_INFO = {
  conflict:        { label: "Conflict",         tone: "danger",  status: "Resolve before continuing" },
  not_compatible:  { label: "Not compatible",   tone: "danger",  status: "Resolve before continuing" },
  mismatch:        { label: "Mismatch",         tone: "danger",  status: "Resolve before continuing" },
  order_block:     { label: "Install Order",    tone: "warn",    status: "Warning only" },
  missing_dep:     { label: "Missing Dep",      tone: "info",    status: "Warning only" },
  path_requirement:{ label: "Path Requirement", tone: "info",    status: "Warning only" },
  conditional:     { label: "Conditional",      tone: "info",    status: "Warning only" },
  deprecated:      { label: "Deprecated",       tone: "warn",    status: "Warning only" },
  warning:         { label: "Warning",          tone: "warn",    status: "Warning only" },
  included:        { label: "Included",         tone: "neutral", status: null },
  not_needed:      { label: "Not needed",       tone: "neutral", status: null },
};
const COMPAT_FILTERS = ["All", "Conflict", "Order", "Mismatch", "Missing", "Included", "Path", "Conditional", "Deprecated", "Warning", "Other"];
const COMPAT_FILTER_TO_KINDS = {
  All:         null,
  Conflict:    ["conflict", "not_compatible"],
  Order:       ["order_block"],
  Mismatch:    ["mismatch"],
  Missing:     ["missing_dep"],
  Included:    ["included", "not_needed"],
  Path:        ["path_requirement"],
  Conditional: ["conditional"],
  Deprecated:  ["deprecated"],
  Warning:     ["warning"],
  Other:       [],
};

// Sample compat-issue corpus per tab. Used by aggregate flows to populate Next-cycle through the toolbar badge.
const MOCK_COMPAT_ISSUES_BGEE = [
  { tp2: "EEFIXPACK", id: 5, label: "Drow Item Restorations", kind: "mismatch", summary: "Only available on `BG2EE, EET`", reason: "This component targets BG2EE/EET-only content; not applicable on BGEE.", source: "src/core/config/default_step2_compat_rules.toml" },
  { tp2: "BG1UB", id: 16, label: "Creature Corrections", kind: "order_block", summary: "Must install after EEFixPack #0", reason: "BG1UB creature edits depend on EEFixPack core being installed first.", source: "step2_compat_rules_default.toml", relatedMod: "EEFIXPACK", relatedComponent: 0 },
  { tp2: "BG1AERIE", id: 5000, label: "Aerie for BG:EE", kind: "missing_dep", summary: "Requires BG1NPC", reason: "Aerie's banters need BG1NPC's content; install BG1NPC first.", source: "step2_compat_rules_default.toml", relatedMod: "BG1NPC", relatedComponent: 0 },
  { tp2: "BG1UB", id: 19, label: "Minor Dialogue Restorations", kind: "warning", summary: "May conflict with other dialogue mods", reason: "Be aware of stacking with BG1 Re-Romanced.", source: "step2_compat_rules_default.toml" },
];
const MOCK_COMPAT_ISSUES_BG2EE = [
  { tp2: "EEUITWEAKS", id: 1020, label: "Mr2150's Random PC Generator", kind: "conflict", summary: "Conflicts with EEUITweaks #1042 (Backup M_BG.lua)", reason: "Both components edit the same M_BG.lua override; later wins. Pick one.", source: "src/core/config/default_step2_compat_rules.toml", relatedMod: "EEUITWEAKS", relatedComponent: 1042 },
  { tp2: "EEUITWEAKS", id: 1070, label: "Faydark's Abilities Auto-Roller", kind: "conflict", summary: "Conflicts with EEUITweaks Mods Options auto-roll", reason: "Two components override the abilities-roll lua. Stacking yields undefined behavior.", source: "step2_compat_rules_default.toml", relatedMod: "EEUITWEAKS", relatedComponent: 1000 },
  { tp2: "EEUITWEAKS", id: 2060, label: "lefreut's 1.3-ish Dialog Box", kind: "conflict", summary: "Conflicts with LeUI core (#0)", reason: "Both modify the dialog UI; install LeUI first then leave this off, or vice versa.", source: "step2_compat_rules_default.toml", relatedMod: "LEUI", relatedComponent: 0 },
  { tp2: "STRATAGEMS", id: 4222, label: "Insane difficulty", kind: "conditional", summary: "Requires parent group selection", reason: "Subcomponent under Improved Watcher's Keep Maze · difficulty. Pick one variant.", source: "step2_compat_rules_default.toml" },
];
const MOCK_COMPAT_ISSUES_BY_TAB = { bgee: MOCK_COMPAT_ISSUES_BGEE, bg2ee: MOCK_COMPAT_ISSUES_BG2EE, iwdee: [] };

const CompatPopup = ({ open, onClose, initialFilter = "All", initialItem, gameTab = "bgee", mode = "single" }) => {
  const tabIssues = MOCK_COMPAT_ISSUES_BY_TAB[gameTab] || [];
  // The "current" issue starts as initialItem (for single mode or single-row clicks),
  // or the first issue matching initialFilter for aggregate mode.
  const [filter, setFilter] = useState(initialFilter);
  const matchesFilter = (it) => {
    const kinds = COMPAT_FILTER_TO_KINDS[filter];
    if (kinds === null) return true; // "All"
    if (kinds.length === 0) return !COMPAT_KIND_INFO[it.kind]; // "Other"
    return kinds.includes(it.kind);
  };
  const filtered = tabIssues.filter(matchesFilter);
  const [currentIdx, setCurrentIdx] = useState(0);
  const [blockOpen, setBlockOpen] = useState(false);
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  React.useEffect(() => {
    if (!open) return;
    setFilter(initialFilter);
    setBlockOpen(false);
    if (initialItem) {
      const idx = tabIssues.findIndex((it) => it.tp2 === initialItem.tp2 && it.id === initialItem.id);
      setCurrentIdx(idx >= 0 ? idx : 0);
    } else {
      setCurrentIdx(0);
    }
  }, [open, initialFilter, initialItem]);

  if (!open) return null;
  // If filter has 0 matches, fall back to all
  const list = filtered.length > 0 ? filtered : tabIssues;
  const current = list[currentIdx % list.length] || tabIssues[0];
  if (!current) {
    return (
      <div onClick={onClose} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
        <div ref={popupRef} onClick={(e) => e.stopPropagation()} style={{ ...sketchyBorder, background: "var(--shell-bg)", boxShadow: "5px 5px 0 var(--shadow)", padding: 22, width: "min(620px, 94%)", ...anchorStyle }}>
          <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 10 }}>
            <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
            <Label style={{ fontSize: 18, fontWeight: 500 }}>Compatibility</Label>
          </div>
          {!collapsed && <>
            <Label style={{ color: "var(--text-faint)" }}>No compatibility issues on this tab.</Label>
            <div style={{ marginTop: 14, display: "flex", justifyContent: "flex-end" }}>
              <Btn small onClick={onClose}>Close</Btn>
            </div>
          </>}
        </div>
      </div>
    );
  }

  const info = COMPAT_KIND_INFO[current.kind] || { label: current.kind, tone: "neutral", status: null };
  const statusColor = info.tone === "danger" ? "#e69a96" : info.tone === "warn" ? "#e8c441" : info.tone === "info" ? "#a8d2cc" : "var(--text-muted)";
  const next = () => setCurrentIdx((i) => (i + 1) % list.length);
  const canJumpRelated = !!(current.relatedMod || current.relatedComponent != null);
  const sampleBlock = `BEGIN ~${current.label}~\n  REQUIRE_PREDICATE GAME_IS ~${current.kind === "mismatch" ? "bg2ee" : "bgee"}~ ~Not compatible~\n  // … component body …`;

  return (
    <div onClick={onClose} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
      <div ref={popupRef} onClick={(e) => e.stopPropagation()} style={{ ...sketchyBorder, background: "var(--shell-bg)", boxShadow: "5px 5px 0 var(--shadow)", width: "min(620px, 94%)", maxHeight: "82vh", display: "flex", flexDirection: "column", padding: 0, ...anchorStyle }}>
        {/* Header: component identification only (BIO compat_popup_step2.rs:87-97).
            No kind Pill, no tp2 sub-line — those live in the body rows. */}
        <div style={{ padding: "14px 18px 8px", borderBottom: collapsed ? "none" : "1.5px dashed var(--border-soft)", display: "flex", alignItems: "center" }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 16, fontWeight: 600 }}>#{current.id} {current.label}</Label>
        </div>

        {!collapsed && <>
        {/* Body: details */}
        <div style={{ padding: "10px 18px", overflow: "auto", flex: 1 }}>
          {info.status && (
            <div style={{ marginBottom: 8 }}>
              <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Status</Label>
              <Label style={{ color: statusColor, fontWeight: 500 }}>{info.status}</Label>
            </div>
          )}
          <div style={{ marginBottom: 8 }}>
            <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Kind</Label>
            <Label>{info.label}</Label>
          </div>
          {current.summary && (
            <div style={{ marginBottom: 8 }}>
              <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Summary</Label>
              <Label style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 13 }}>{current.summary}</Label>
            </div>
          )}
          {current.reason && (
            <div style={{ marginBottom: 8 }}>
              <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Reason</Label>
              <Label style={{ color: "var(--text)", lineHeight: 1.45 }}>{current.reason}</Label>
            </div>
          )}
          {(current.relatedMod || current.relatedComponent != null) && (
            <div style={{ marginBottom: 8 }}>
              <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Related</Label>
              <Label style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 13 }}>
                {current.relatedMod}{current.relatedComponent != null ? ` #${current.relatedComponent}` : ""}
              </Label>
            </div>
          )}
          {current.source && (
            <div style={{ marginBottom: 8 }}>
              <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Rule source</Label>
              <Label style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 12, color: "var(--text-faint)" }}>{current.source}</Label>
            </div>
          )}
          <div style={{ marginTop: 12 }}>
            <div onClick={() => setBlockOpen((v) => !v)} style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", userSelect: "none" }}>
              <span style={{ color: "var(--text-faint)", fontSize: 11 }}>{blockOpen ? "▾" : "▸"}</span>
              <Label hand style={{ color: "var(--text-muted)", fontSize: 12 }}>Component block</Label>
            </div>
            {blockOpen && (
              <div style={{ ...sketchyBorder, padding: 8, marginTop: 4, background: "var(--input-bg)", fontFamily: "'FiraCode Nerd', monospace", fontSize: 11, color: "var(--text)", whiteSpace: "pre-wrap", maxHeight: 160, overflow: "auto" }}>
                {sampleBlock}
              </div>
            )}
          </div>

          {/* Filter row */}
          <div style={{ marginTop: 14 }}>
            <Label hand style={{ color: "var(--text-muted)", fontSize: 12, marginBottom: 6 }}>Filter by category</Label>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {COMPAT_FILTERS.map((f) => {
                const kinds = COMPAT_FILTER_TO_KINDS[f];
                const count = f === "All" ? tabIssues.length
                  : f === "Other" ? tabIssues.filter((it) => !COMPAT_KIND_INFO[it.kind]).length
                  : tabIssues.filter((it) => kinds && kinds.includes(it.kind)).length;
                const isActive = filter === f;
                const disabled = count === 0 && f !== "All";
                return (
                  <Btn
                    key={f}
                    small
                    primary={isActive}
                    disabled={disabled}
                    onClick={() => { setFilter(f); setCurrentIdx(0); }}
                    style={{ fontSize: 12 }}
                  >
                    {f}{count > 0 && f !== "All" ? ` ${count}` : ""}
                  </Btn>
                );
              })}
            </div>
          </div>
        </div>

        {/* Footer: action row */}
        <div style={{ padding: "10px 18px", borderTop: "1.5px dashed var(--border-soft)", display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <Btn small>Jump To This</Btn>
          <Btn small disabled={!canJumpRelated}>Jump To Related</Btn>
          <Btn small onClick={next} disabled={list.length <= 1}>Next →</Btn>
          <Btn small disabled={!current.source}>Open Rule Source</Btn>
          <div style={{ marginLeft: "auto" }}>
            <Btn small onClick={onClose}>Close</Btn>
          </div>
        </div>
        </>}
      </div>
    </div>
  );
};

// Prompt popup — matches BIO's prompt window (prompt_popup_step2.rs).
// Two modes: single (per-component, parsed prompt summary + jump buttons)
// and aggregate (toolbar — collapsible list of mods with component-id jump buttons).
const MOCK_PROMPT_BODY = (c, m) => `Prompt summary from Lapdu parser:

Component: ${c.id} - ${c.name}

The component asks:
  "Choose installation method:"
    - [V] Vanilla — preserves the original visuals
    - [E] Enhanced — applies the new lua + portrait overrides
    - [B] Both — installs Vanilla and Enhanced side-by-side

  "Apply patch to existing files? (y/n)"
    - Y = patch in place
    - N = skip patching

  "Confirm install location: D:\\BG2EE [Y/n]"
    - Y = use D:\\BG2EE
    - n = abort

BIO will auto-answer based on the prompt eval rules. Component ${(c.id || 0) - 1} sets up shared state used by this prompt.`;

const PromptPopup = ({ open, onClose, mode = "single", component, mod, gameTab = "bgee" }) => {
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  if (!open) return null;
  // SINGLE mode: parsed prompts for one component
  if (mode === "single" && component) {
    const c = component;
    const m = mod || { name: c.modName, tp2: c.tp2, version: c.version };
    const jumpIds = [(c.id || 0) - 1, (c.id || 0) + 10].filter((n) => n > 0); // demo jump targets
    return (
      <div onClick={onClose} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
        <div ref={popupRef} onClick={(e) => e.stopPropagation()} style={{ ...sketchyBorder, background: "var(--shell-bg)", boxShadow: "5px 5px 0 var(--shadow)", width: "min(700px, 94%)", maxHeight: "82vh", display: "flex", flexDirection: "column", padding: 0, ...anchorStyle }}>
          {/* Header: single literal title matching BIO (prompt_popup_step2.rs:24).
              No Pill, no separate sub-line — just the window title. */}
          <div style={{ padding: "14px 18px 8px", borderBottom: collapsed ? "none" : "1.5px dashed var(--border-soft)", display: "flex", alignItems: "center" }}>
            <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
            <Label style={{ fontSize: 16, fontWeight: 600, fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif" }}>
              Parsed prompts - <span style={{ fontFamily: "'FiraCode Nerd', monospace", fontSize: 14 }}>{(m.tp2 || c.tp2 || "").toLowerCase()}.tp2 #{c.id}</span>
            </Label>
          </div>
          {!collapsed && <>
          {/* Body */}
          <div style={{ padding: "12px 18px", overflow: "auto", flex: 1 }}>
            <Label hand style={{ color: "var(--text-muted)", fontSize: 12, marginBottom: 6 }}>Prompt summary from Lapdu parser:</Label>
            <div style={{ ...sketchyBorder, padding: 10, background: "var(--input-bg)", fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif", fontSize: 13, color: "var(--text)", lineHeight: 1.5, whiteSpace: "pre-wrap" }}>
              {MOCK_PROMPT_BODY(c, m)}
            </div>
            {jumpIds.length > 0 && (
              <>
                <Label hand style={{ color: "var(--text-muted)", fontSize: 12, marginTop: 14, marginBottom: 4 }}>Jump to component</Label>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                  {jumpIds.map((id) => (
                    <Btn
                      key={id}
                      small
                      style={{
                        fontFamily: "'FiraCode Nerd', monospace",
                        color: "#2f6fb7",
                        minWidth: 42,
                      }}
                    >#{id}</Btn>
                  ))}
                </div>
              </>
            )}
          </div>
          {/* Footer */}
          <div style={{ padding: "10px 18px", borderTop: "1.5px dashed var(--border-soft)", display: "flex", justifyContent: "flex-end" }}>
            <Btn small onClick={onClose}>Close</Btn>
          </div>
          </>}
        </div>
      </div>
    );
  }

  // AGGREGATE mode: list of mods on the tab with prompt counts
  const mods = MOCK_MODS_BY_TAB[gameTab] || [];
  const entries = mods.map((mm) => {
    const promptLeaves = flattenComponents(mm.components).filter((c) => c.prompt && c.checked);
    return promptLeaves.length > 0 ? { tp2: mm.tp2, name: mm.name, ids: promptLeaves.map((c) => c.id) } : null;
  }).filter(Boolean);
  return (
    <div onClick={onClose} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
      <div ref={popupRef} onClick={(e) => e.stopPropagation()} style={{ ...sketchyBorder, background: "var(--shell-bg)", boxShadow: "5px 5px 0 var(--shadow)", width: "min(420px, 94%)", maxHeight: "82vh", display: "flex", flexDirection: "column", padding: 0, ...anchorStyle }}>
        {/* Header: single literal title matching BIO (toolbar_actions_step2.rs:82-86). */}
        <div style={{ padding: "14px 18px 8px", borderBottom: collapsed ? "none" : "1.5px dashed var(--border-soft)", display: "flex", alignItems: "center" }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 16, fontWeight: 600 }}>
            Prompt Components ({gameTab.toUpperCase()})
          </Label>
        </div>
        {!collapsed && <>
        <div style={{ padding: "10px 18px", overflow: "auto", flex: 1 }}>
          {entries.length === 0 ? (
            <Label style={{ color: "var(--text-faint)" }}>No components with prompts on this tab.</Label>
          ) : (
            entries.map((e) => <PromptModEntry key={e.tp2} entry={e} />)
          )}
        </div>
        <div style={{ padding: "10px 18px", borderTop: "1.5px dashed var(--border-soft)", display: "flex", justifyContent: "flex-end" }}>
          <Btn small onClick={onClose}>Close</Btn>
        </div>
        </>}
      </div>
    </div>
  );
};

const PromptModEntry = ({ entry }) => {
  const [open, setOpen] = useState(true);
  return (
    <div style={{ marginBottom: 6 }}>
      <div onClick={() => setOpen((v) => !v)} style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", padding: "4px 6px", background: "var(--rail-bg)", ...sketchyBorder }}>
        <span style={{ color: "var(--text-faint)", fontSize: 11 }}>{open ? "▾" : "▸"}</span>
        <Label style={{ fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif", fontSize: 13 }}>
          {entry.name} <span style={{ color: "var(--text-faint)", fontWeight: 400 }}>({entry.ids.length})</span>
        </Label>
      </div>
      {open && (
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", padding: "6px 8px" }}>
          {entry.ids.map((id) => (
            <Btn key={id} small style={{ fontFamily: "'FiraCode Nerd', monospace", color: "#2f6fb7", minWidth: 42 }}>#{id}</Btn>
          ))}
        </div>
      )}
    </div>
  );
};

const PillPopup = ({ open, title, body, onClose }) => {
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  if (!open) return null;
  return (
    <div
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "5px 5px 0 var(--shadow)",
          padding: 18,
          maxWidth: 520,
          width: "92%",
          ...anchorStyle,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 8 }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 15, fontWeight: 500 }}>{title}</Label>
        </div>
        {!collapsed && <>
        <div style={{ color: "var(--text-muted)", marginBottom: 14, fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap", fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif" }}>
          {body}
        </div>
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <Btn small onClick={onClose}>Close</Btn>
        </div>
        </>}
      </div>
    </div>
  );
};

// Fork lineage popup (SPEC §10.9). Read-only credit chain: oldest → newest
// ancestors from `lineage`, culminating in the current modlist (`self`).
// Triggered from the workspace header `⑂ view fork details` and the
// Install / Fork preview `⑂ fork info` affordance.
const ForkInfoPopup = ({ open, onClose, lineage = [], self }) => {
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  if (!open) return null;
  const hasLineage = Array.isArray(lineage) && lineage.length > 0;
  const chain = hasLineage
    ? [...lineage.map((a) => ({ ...a, current: false })), ...(self ? [{ ...self, current: true }] : [])]
    : [];
  return (
    <div
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "5px 5px 0 var(--shadow)",
          padding: 18,
          maxWidth: 480,
          width: "92%",
          ...anchorStyle,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 14 }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 15, fontWeight: 500 }}>Fork lineage</Label>
        </div>
        {!collapsed && <>
          {!hasLineage ? (
            <Label hand style={{ color: "var(--text-faint)", fontSize: 13, marginBottom: 16, display: "block" }}>
              This modlist was created from scratch — no fork lineage.
            </Label>
          ) : (
            <div style={{ marginBottom: 16 }}>
              {chain.map((node, i) => (
                <div key={i} style={{ marginLeft: i * 20, marginBottom: i === chain.length - 1 ? 0 : 10 }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                    {i > 0 && (
                      <span style={{ fontFamily: "'FiraCode Nerd', monospace", color: "var(--text-faint)", fontSize: 13 }}>↳</span>
                    )}
                    <span style={{
                      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                      fontSize: 14,
                      fontWeight: node.current ? 700 : 500,
                      color: node.current ? "var(--accent-deep)" : "var(--text)",
                    }}>
                      {node.name}
                    </span>
                    {node.current && (
                      <span style={{
                        ...sketchyBorder,
                        padding: "1px 7px",
                        fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                        fontSize: 9,
                        fontWeight: 500,
                        letterSpacing: 1,
                        textTransform: "uppercase",
                        color: "var(--text-muted)",
                        whiteSpace: "nowrap",
                      }}>⑂ this modlist</span>
                    )}
                  </div>
                  <div style={{
                    marginLeft: i > 0 ? 21 : 0,
                    fontFamily: "'FiraCode Nerd', monospace",
                    fontSize: 12,
                    color: "var(--text-faint)",
                    marginTop: 2,
                  }}>
                    by {node.author || "—"}
                  </div>
                </div>
              ))}
            </div>
          )}
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <Btn small onClick={onClose}>Close</Btn>
          </div>
        </>}
      </div>
    </div>
  );
};

// Mock data for the Updates popup demo state (matches BIO's category model).
// Source Choices lists every mod on the active tab that has at least one configured source.
// Even mods with a single source appear here with a dropdown showing that lone option.
const UPDATES_MOCK = {
  sourceChoices: [
    { tp2: "DLCMERGER", label: "DLCMerger", currentSourceId: "argent77", options: ["argent77"] },
    { tp2: "EEFIXPACK", label: "EEFixPack", currentSourceId: "gibberlings3", options: ["gibberlings3", "weasel-fork"] },
    { tp2: "BG1UB", label: "BG1 Unfinished Business", currentSourceId: "pocket-plane-group", options: ["pocket-plane-group", "gibberlings3-fork"] },
    { tp2: "BG1AERIE", label: "BG1Aerie", currentSourceId: "spellhold", options: ["spellhold"] },
    { tp2: "EEUITWEAKS", label: "EEUITweaks", currentSourceId: "r-e-d", options: ["r-e-d", "gibberlings3"] },
    { tp2: "STRATAGEMS", label: "Sword Coast Stratagems", currentSourceId: "gibberlings3", options: ["gibberlings3", "weasel-fork", "github-mirror"] },
    { tp2: "LEUI", label: "LeUI", currentSourceId: "lefreut", options: ["lefreut"] },
    { tp2: "CDTWEAKS", label: "Tweaks Anthology", currentSourceId: "gibberlings3", options: ["gibberlings3"] },
    { tp2: "EET", label: "EET", currentSourceId: "k4thos", options: ["k4thos", "gibberlings3-fork"] },
    { tp2: "EEEX", label: "EEex", currentSourceId: "bubb", options: ["bubb"] },
  ],
  updates: [
    { tp2: "EEUITWEAKS", label: "EEUITweaks" },
    { tp2: "STRATAGEMS", label: "Sword Coast Stratagems" },
  ],
  manual: [
    { tp2: "BG1NPC", label: "BG1 NPC Project" },
    { tp2: "BG2TWEAKS", label: "BG2 Tweaks" },
  ],
  missing: [
    { tp2: "CDTWEAKS-EXTRAS", label: "cdtweaks-extras" },
  ],
  downloaded: [
    { tp2: "BG1UB", label: "BG1 Unfinished Business" },
  ],
  failedDownload: [],
  extracted: [
    { tp2: "BG1UB", label: "BG1 Unfinished Business" },
  ],
  failedExtract: [],
  failed: [
    { tp2: "STRATAGEMS-FORK", label: "stratagems-fork" },
  ],
};

// Updates popup — behavior matches BIO's update-check window exactly
// (src/ui/step2/update_check/update_check_popup_step2.rs). Only colors and fonts differ.
// Same body structure: optional source-choices grid → status text → category sections in fixed order.
// Per-row action is "Edit Source" everywhere except no-source/missing sections, which use "Add Mod".
// No tone-coding on sections, no version transitions in list rows, no per-row "Open" actions —
// all of those are not in BIO.
const UpdatesPopup = ({ open, onClose, exactLogMode = false }) => {
  const [checkRun, setCheckRun] = useState(true); // demo: post-check state
  const [checking, setChecking] = useState(false);
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  if (!open) return null;
  const title = exactLogMode ? "Check Mod List" : "Check Updates";
  const checkLabel = exactLogMode ? "Check Mod List" : "Check Updates";
  const downloadLabel = exactLogMode ? "Download Missing Mods" : "Download Updates";

  // BIO status strings (verbatim phrasing).
  const total = UPDATES_MOCK.updates.length + UPDATES_MOCK.manual.length + UPDATES_MOCK.missing.length;
  const status = checking
    ? (exactLogMode
        ? `Checking missing mod sources 3/${total}`
        : `Checking updates / missing mod sources 3/${total}`)
    : checkRun
      ? null
      : (exactLogMode ? null : "No update check run yet.");

  const startCheck = () => {
    setChecking(true);
    setTimeout(() => { setChecking(false); setCheckRun(true); }, 1100);
  };

  // BIO section: bold label + count in a flat info-fill header, then a 2-col grid (mod name | action button).
  // Single uniform background tint for all section headers (BIO does not tone-code sections).
  const sectionBox = (label, rows, action = "Edit Source") => {
    if (!rows || rows.length === 0) return null;
    return (
      <div style={{ marginTop: 10 }}>
        <div style={{
          padding: "4px 10px",
          background: "var(--rail-bg)",
          ...sketchyBorder,
          fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
          fontSize: 12,
          fontWeight: 500,
        }}>{label} <span style={{ color: "var(--text-faint)", fontWeight: 400 }}>({rows.length})</span></div>
        <div style={{
          display: "grid",
          gridTemplateColumns: "1fr auto",
          gap: "4px 12px",
          padding: "6px 10px",
          ...sketchyBorder,
          borderTop: "none",
          fontSize: 13,
        }}>
          {rows.map((r) => (
            <React.Fragment key={r.tp2}>
              <Label style={{ alignSelf: "center" }}>{r.label}</Label>
              <Btn small disabled={checking}>{action}</Btn>
            </React.Fragment>
          ))}
        </div>
      </div>
    );
  };

  return (
    <div
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.55)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "5px 5px 0 var(--shadow)",
          width: "min(780px, 94%)",
          maxHeight: "82vh",
          display: "flex",
          flexDirection: "column",
          padding: 0,
          ...anchorStyle,
        }}
      >
        {/* header: title only (BIO does not show tab name or status text here). */}
        <div style={{
          padding: "14px 18px 10px",
          borderBottom: collapsed ? "none" : "1.5px dashed var(--border-soft)",
          display: "flex",
          alignItems: "center",
        }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 18, fontWeight: 500 }}>{title}</Label>
        </div>

        {!collapsed && <>
        {/* body */}
        <div style={{ padding: "10px 18px", overflow: "auto", flex: 1 }}>
          {/* status text (top of body, BIO position) */}
          {status && (
            <Label style={{ color: checking ? "var(--success)" : "var(--text-muted)", fontSize: 13, marginBottom: 8 }}>
              {status}
            </Label>
          )}

          {/* Source Choices: BIO lists EVERY mod on the active tab that has at least one
              configured source. Each row has its current source dropdown + per-row actions. */}
          {UPDATES_MOCK.sourceChoices.length > 0 && (
            <div style={{ marginTop: 4, marginBottom: 6 }}>
              <div style={{
                padding: "4px 10px",
                background: "var(--rail-bg)",
                ...sketchyBorder,
                fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                fontSize: 12,
                fontWeight: 500,
              }}>Source Choices <span style={{ color: "var(--text-faint)", fontWeight: 400 }}>({UPDATES_MOCK.sourceChoices.length})</span></div>
              <div style={{
                display: "grid",
                gridTemplateColumns: "1fr 180px auto auto auto",
                gap: "6px 10px",
                padding: "8px 10px",
                ...sketchyBorder,
                borderTop: "none",
                fontSize: 13,
                alignItems: "center",
              }}>
                {UPDATES_MOCK.sourceChoices.map((sc) => (
                  <React.Fragment key={sc.tp2}>
                    <Label>{sc.label}</Label>
                    <select
                      defaultValue={sc.currentSourceId}
                      style={{
                        ...sketchyBorder,
                        padding: "3px 8px",
                        background: "var(--input-bg)",
                        fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                        fontSize: 12,
                        color: "var(--text)",
                        outline: "none",
                      }}
                    >
                      {sc.options.map((o) => <option key={o} value={o}>{o}</option>)}
                    </select>
                    <Btn small disabled={checking}>Edit Source</Btn>
                    <Btn small disabled={checking}>Open Source</Btn>
                    <Btn small disabled={checking}>Discover Forks</Btn>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}

          {(checkRun || checking) && (
            <>
              {sectionBox(exactLogMode ? "Downloadable Missing Mods" : "Updates", UPDATES_MOCK.updates)}
              {sectionBox("Manual Sources", UPDATES_MOCK.manual)}
              {sectionBox("No Source Entries", UPDATES_MOCK.missing, "Add Mod")}
              {sectionBox("Source Check Failed", UPDATES_MOCK.failed)}
              {sectionBox("Downloaded", UPDATES_MOCK.downloaded)}
              {sectionBox("Download Failed", UPDATES_MOCK.failedDownload)}
              {sectionBox("Extracted", UPDATES_MOCK.extracted)}
              {sectionBox("Extract Failed", UPDATES_MOCK.failedExtract)}
            </>
          )}
        </div>

        {/* footer */}
        <div style={{
          padding: "12px 18px",
          borderTop: "1.5px dashed var(--border-soft)",
          display: "flex",
          gap: 8,
          flexWrap: "wrap",
          alignItems: "center",
        }}>
          <Btn small primary onClick={startCheck} disabled={checking}>{checkLabel}</Btn>
          <Btn small>Add Source</Btn>
          <Btn small disabled={!checkRun}>Copy Report</Btn>
          <Btn small primary disabled={!checkRun || UPDATES_MOCK.updates.length === 0}>{downloadLabel}</Btn>
          <div style={{ marginLeft: "auto" }}>
            <Btn small onClick={onClose}>Close</Btn>
          </div>
        </div>
        </>}
      </div>
    </div>
  );
};

const LoadDraftDialog = ({ open, onCancel, onResume }) => {
  const [toast, setToast] = useState("");
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  if (!open) return null;
  const builds = MOCK_IN_PROGRESS_BUILDS;
  const copyPasteCode = (modlistName) => {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(SAMPLE_PASTE_CODE).catch(() => {});
    }
    setToast(`Copied import code for "${modlistName}"`);
    setTimeout(() => setToast(""), 1600);
  };
  return (
    <div
      onClick={onCancel}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.55)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "5px 5px 0 var(--shadow)",
          padding: 22,
          maxWidth: 620,
          width: "94%",
          ...anchorStyle,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 6 }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 18, fontWeight: 500 }}>Resume in-progress build</Label>
        </div>
        {!collapsed && <>
        <Label style={{ color: "var(--text-muted)", marginBottom: 16, fontSize: 13, lineHeight: 1.5 }}>
          Pick a build to resume. BIO restores its order, selection, and settings and drops you back where you left off.
        </Label>

        {builds.length === 0 ? (
          <Box style={{ padding: "16px 20px", marginBottom: 14 }}>
            <Label style={{ color: "var(--text-faint)" }}>No in-progress builds. Start a new modlist from Create.</Label>
          </Box>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 10, marginBottom: 14 }}>
            {builds.map((m) => (
              <Box key={m.id} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 12px" }}>
                <div>
                  <Label>{m.n}</Label>
                  <Label hand style={{ fontSize: "14px", color: "var(--text-faint)" }}>{m.meta}</Label>
                </div>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <Btn small primary onClick={() => onResume && onResume(m)}>resume</Btn>
                  <Kebab items={[
                    { label: "Copy import code", onClick: () => copyPasteCode(m.n) },
                    { label: "Delete", onClick: () => {} },
                  ]} />
                </div>
              </Box>
            ))}
          </div>
        )}

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <Btn small onClick={onCancel}>Cancel</Btn>
        </div>
        {toast && (
          <div style={{
            marginTop: 10,
            ...sketchyBorder,
            background: "var(--shell-bg)",
            padding: "6px 12px",
            fontSize: 13,
            color: "var(--success)",
          }}>✓ {toast}</div>
        )}
        </>}
      </div>
    </div>
  );
};

const SaveDraftDialog = ({ open, onCancel, onSaved }) => {
  const [folder, setFolder] = useState("");
  const [filename, setFilename] = useState("modlist-draft");
  const [saved, setSaved] = useState(false);
  const [collapsed, toggleCollapsed, popupRef, anchorStyle] = usePopupCollapse(open);
  React.useEffect(() => {
    if (open) { setSaved(false); }
  }, [open]);
  if (!open) return null;
  const handleBrowse = () => setFolder("D:\\BIO\\drafts");
  const handleSave = () => {
    setSaved(true);
    setTimeout(() => { onSaved && onSaved(); }, 1100);
  };
  const fullPath = folder ? `${folder}\\${filename}.txt` : `${filename}.txt`;
  return (
    <div
      onClick={onCancel}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.55)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
    >
      <div
        ref={popupRef}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...sketchyBorder,
          background: "var(--shell-bg)",
          boxShadow: "5px 5px 0 var(--shadow)",
          padding: 22,
          maxWidth: 560,
          width: "94%",
          ...anchorStyle,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", marginBottom: collapsed ? 0 : 6 }}>
          <PopupCollapseChevron collapsed={collapsed} onClick={toggleCollapsed} />
          <Label style={{ fontSize: 18, fontWeight: 500 }}>Save draft</Label>
        </div>
        {!collapsed && <>
        <Label style={{ color: "var(--text-muted)", marginBottom: 16, fontSize: 13, lineHeight: 1.5 }}>
          Saves a .txt file with this modlist's BIO share code. Load it later from the Install screen.
        </Label>

        <div style={{ marginBottom: 12 }}>
          <FolderInput
            label="save to folder"
            placeholder="D:\BIO\drafts"
            value={folder}
            onBrowse={handleBrowse}
          />
        </div>

        <div style={{ marginBottom: 14 }}>
          <Label hand style={{ marginBottom: 4, color: "var(--text-muted)" }}>file name</Label>
          <div style={{ display: "flex", alignItems: "stretch", gap: 0 }}>
            <input
              value={filename}
              onChange={(e) => setFilename(e.target.value)}
              placeholder="modlist-draft"
              style={{
                ...sketchyBorder,
                padding: "8px 12px",
                background: "var(--input-bg)",
                fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                fontSize: 14,
                color: "var(--text)",
                flex: 1,
                outline: "none",
                borderRight: "none",
                borderRadius: "3px 0 0 3px",
              }}
            />
            <span style={{
              ...sketchyBorder,
              padding: "8px 12px",
              background: "var(--chrome-bg)",
              fontFamily: "'FiraCode Nerd', monospace",
              fontSize: 13,
              color: "var(--text-muted)",
              borderLeft: "none",
              borderRadius: "0 3px 3px 0",
              display: "flex",
              alignItems: "center",
            }}>.txt</span>
          </div>
          <Label hand style={{ fontSize: 12, color: "var(--text-faint)", marginTop: 4 }}>
            full path: <span style={{ fontFamily: "'FiraCode Nerd', monospace" }}>{fullPath}</span>
          </Label>
        </div>

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", alignItems: "center" }}>
          {saved && (
            <Label hand style={{ color: "var(--success)", marginRight: "auto", fontSize: 14 }}>
              ✓ saved
            </Label>
          )}
          <Btn small onClick={onCancel} disabled={saved}>Cancel</Btn>
          <Btn small primary onClick={handleSave} disabled={!folder || !filename.trim() || saved}>Save</Btn>
        </div>
        </>}
      </div>
    </div>
  );
};

const SourcesPanel = ({ gameTab, setGameTab, fork, game = "EET", orderByTab, setOrderByTab }) => {
  const tabs = tabsForGame(game);
  const upperTab = (tabs.find((t) => t.id === gameTab) || tabs[0]).label;
  const mods = MOCK_MODS_BY_TAB[gameTab] || [];
  const totalComponents = mods.reduce((acc, m) => acc + flattenComponents(m.components).length, 0);
  const checkedKeys = new Set((orderByTab[gameTab] || []).map((c) => `${c.tp2}:${c.id}`));
  const selectedComponents = checkedKeys.size;
  const [confirm, setConfirm] = useState(null); // null | { title, message, confirmLabel, onConfirm }
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [selectedKey, setSelectedKey] = useState(null);
  const [selectedItem, setSelectedItem] = useState(null);
  const [pillPopup, setPillPopup] = useState(null); // null | { title, body }
  const [compatPopup, setCompatPopup] = useState(null); // null | { mode, initialFilter, initialItem }
  const [promptPopup, setPromptPopup] = useState(null); // null | { mode, component, mod }
  const [updatesOpen, setUpdatesOpen] = useState(false);

  const askWeiduImport = () => setConfirm({
    title: `Replace ${upperTab} selections from a WeiDU log?`,
    message: `This will overwrite every component selection on the ${upperTab} bucket with the contents of the chosen weidu.log. Make sure the log was produced from the same mod versions you have downloaded — otherwise components may resolve to the wrong rows or fail to install.`,
    confirmLabel: "Pick a weidu.log...",
    danger: true,
    onConfirm: () => { /* stub */ },
  });

  return (
    <div style={{
      display: "flex",
      flexDirection: "column",
      flex: 1,
      minHeight: 0,
      padding: 0,
    }}>
      <div style={{ flexShrink: 0 }}>
        {/* Title */}
        <Label style={{ fontSize: 15, fontWeight: 500, marginBottom: 8 }}>Mods / Components</Label>

        {/* Search + Rescan */}
        <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 10 }}>
          <Input wide value="Search mods or components..." style={{ flex: 1, background: "var(--input-bg)" }} />
          <TopButton>Rescan Mods Folder</TopButton>
        </div>

        {/* Tab row: tabs sit on top of the tree pane, their bottom merges with the tree pane's top border
            (Settings-style behavior). position:relative + z-index lifts the row above the grid so the
            active tab's shell-bg bottom border masks the tree pane's top border. */}
        <div style={{
          display: "flex",
          alignItems: "flex-start",
          gap: 4,
          position: "relative",
          zIndex: 1,
        }}>
          {tabs.map((t) => (
            <GameTab key={t.id} active={gameTab === t.id} onClick={() => setGameTab(t.id)}>{t.label}</GameTab>
          ))}
          <div style={{
            flex: 1,
            display: "flex",
            alignItems: "center",
            gap: 8,
            paddingLeft: 12,
            paddingRight: 4,
            height: 30,
            boxSizing: "border-box",
            flexWrap: "wrap",
          }}>
            {!fork && <TopButton onClick={askWeiduImport}>Select {upperTab} via WeiDU Log</TopButton>}
            <TopButton onClick={() => setUpdatesOpen(true)}>Updates...</TopButton>
            <Pill
              tone="danger"
              title={`153 compatibility issues in the ${upperTab} Step 2 tab. Active badge category: Mismatch (153). Dominant category: Mismatch (153).`}
              onClick={() => setCompatPopup({ mode: "aggregate", initialFilter: "Mismatch" })}
            >{upperTab} Mismatch 153</Pill>
            <Pill
              tone="warn"
              title="See parsed prompts on the current tab"
              onClick={() => setPromptPopup({ mode: "aggregate" })}
            >PROMPT {mods.flatMap((m) => flattenComponents(m.components)).filter((c) => c.prompt).length}</Pill>
            <Label hand style={{ marginLeft: "auto", color: "var(--text-faint)", fontSize: 12 }}>
              {selectedComponents} / {totalComponents} on {upperTab}
            </Label>
            <Kebab items={[
              { label: detailsOpen ? "Hide Details panel" : "Show Details panel", onClick: () => setDetailsOpen((v) => !v) },
              { label: "Clear All", onClick: () => {} },
              { label: "Select Visible", onClick: () => {} },
              { label: "Collapse All", onClick: () => {} },
              { label: "Expand All", onClick: () => {} },
              { label: "Jump to Selected", onClick: () => {} },
            ]} />
          </div>
        </div>
      </div>
      <div style={{
        display: "grid",
        gridTemplateColumns: detailsOpen ? "1fr 420px" : "1fr",
        gap: 12,
        flex: 1,
        minHeight: 0,
      }}>
        <Box style={{ padding: 10, height: "100%", overflow: "auto", minHeight: 0 }}>
          <ComponentTree
            gameTab={gameTab}
            selectedKey={selectedKey}
            onSelect={(key, item) => { setSelectedKey(key); setSelectedItem(item); }}
            onOpenDetails={() => setDetailsOpen(true)}
            onShowPopup={setPillPopup}
            onShowCompat={setCompatPopup}
            onShowPrompt={setPromptPopup}
            orderByTab={orderByTab}
            setOrderByTab={setOrderByTab}
          />
        </Box>
        {detailsOpen && (
          <DetailsPanel
            selected={selectedItem}
            onClose={() => setDetailsOpen(false)}
            style={{ height: "100%", minHeight: 0, overflowY: "auto", overflowX: "hidden" }}
          />
        )}
      </div>

      <ConfirmDialog
        open={!!confirm}
        title={confirm?.title}
        message={confirm?.message}
        confirmLabel={confirm?.confirmLabel}
        danger={confirm?.danger}
        onConfirm={() => { confirm?.onConfirm?.(); setConfirm(null); }}
        onCancel={() => setConfirm(null)}
      />
      <PillPopup
        open={!!pillPopup}
        title={pillPopup?.title}
        body={pillPopup?.body}
        onClose={() => setPillPopup(null)}
      />
      <CompatPopup
        open={!!compatPopup}
        mode={compatPopup?.mode || "single"}
        initialFilter={compatPopup?.initialFilter || "All"}
        initialItem={compatPopup?.initialItem}
        gameTab={gameTab}
        onClose={() => setCompatPopup(null)}
      />
      <PromptPopup
        open={!!promptPopup}
        mode={promptPopup?.mode || "single"}
        component={promptPopup?.component}
        mod={promptPopup?.mod}
        gameTab={gameTab}
        onClose={() => setPromptPopup(null)}
      />
      <UpdatesPopup
        open={updatesOpen}
        onClose={() => setUpdatesOpen(false)}
      />
    </div>
  );
};

const ComponentsPanel = ({ gameTab, setGameTab, game = "EET", orderByTab, setOrderByTab }) => {
  const tabs = tabsForGame(game);
  const items = orderByTab[gameTab] || [];
  // Compute groups (consecutive same-tp2 runs). The first run of each tp2 is canonical;
  // subsequent runs get a "(copy)" label so users see when components split off.
  const byMod = [];
  const tp2RunCount = new Map();
  items.forEach((c, idx) => {
    const enriched = { ...c, _idx: idx };
    const last = byMod[byMod.length - 1];
    if (last && last.tp2 === c.tp2) {
      last.items.push(enriched);
    } else {
      const count = tp2RunCount.get(c.tp2) || 0;
      tp2RunCount.set(c.tp2, count + 1);
      byMod.push({
        tp2: c.tp2,
        modName: c.modName,
        version: c.version,
        copySuffix: count > 0 ? (count > 1 ? ` (copy ${count})` : " (copy)") : "",
        items: [enriched],
      });
    }
  });
  const conflictCount = items.filter((c) => c.conflict).length;
  const promptCount = items.filter((c) => c.prompt).length;

  const [selectedIds, setSelectedIds] = useState(new Set());
  const [anchorIdx, setAnchorIdx] = useState(null);
  React.useEffect(() => { setSelectedIds(new Set()); setAnchorIdx(null); }, [gameTab]);
  const keyOf = (c) => `${c.tp2}:${c.id}`;
  const handleRowClick = (c, e) => {
    const key = keyOf(c);
    if (e.shiftKey && anchorIdx !== null) {
      const start = Math.min(anchorIdx, c._idx);
      const end = Math.max(anchorIdx, c._idx);
      const next = new Set();
      for (let i = start; i <= end; i++) next.add(keyOf(items[i]));
      setSelectedIds(next);
    } else {
      setSelectedIds(new Set([key]));
      setAnchorIdx(c._idx);
    }
  };

  // Drag state
  const [dragKeys, setDragKeys] = useState([]);
  const [dragOver, setDragOver] = useState(null); // { idx, side: "above" | "below" }
  const handleDragStart = (c) => (e) => {
    e.dataTransfer.effectAllowed = "move";
    const key = keyOf(c);
    let keys;
    if (selectedIds.has(key)) {
      keys = Array.from(selectedIds);
    } else {
      keys = [key];
      setSelectedIds(new Set([key]));
      setAnchorIdx(c._idx);
    }
    setDragKeys(keys);
    e.dataTransfer.setData("text/plain", JSON.stringify(keys));
  };
  const handleDragOver = (idx) => (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    const rect = e.currentTarget.getBoundingClientRect();
    const side = e.clientY < rect.top + rect.height / 2 ? "above" : "below";
    if (!dragOver || dragOver.idx !== idx || dragOver.side !== side) {
      setDragOver({ idx, side });
    }
  };
  const handleDragEnd = () => { setDragKeys([]); setDragOver(null); };
  const handleDrop = (e) => {
    e.preventDefault();
    if (!dragOver || dragKeys.length === 0) { handleDragEnd(); return; }
    const moveSet = new Set(dragKeys);
    const dropTarget = items[dragOver.idx];
    if (moveSet.has(keyOf(dropTarget))) { handleDragEnd(); return; }
    const moved = items.filter((c) => moveSet.has(keyOf(c)));
    const remaining = items.filter((c) => !moveSet.has(keyOf(c)));
    const remDropIdx = remaining.findIndex((c) => keyOf(c) === keyOf(dropTarget));
    const insertIdx = dragOver.side === "below" ? remDropIdx + 1 : remDropIdx;
    const newItems = [...remaining.slice(0, insertIdx), ...moved, ...remaining.slice(insertIdx)];
    setOrderByTab((prev) => ({ ...prev, [gameTab]: newItems }));
    handleDragEnd();
  };
  const dropLine = (
    <div style={{ height: 2, background: "var(--accent)", margin: "0 4px", borderRadius: 1 }} />
  );

  // Group collapse state. Keyed by `${tp2}:${first-item-id}` so it stays stable across reorders
  // as long as the group's first item doesn't change.
  const [collapsedGroups, setCollapsedGroups] = useState(new Set());
  const groupIdOf = (g) => `${g.tp2}:${g.items[0].id}`;
  const isGroupCollapsed = (g) => collapsedGroups.has(groupIdOf(g));
  const toggleGroupCollapsed = (g) => setCollapsedGroups((s) => {
    const next = new Set(s);
    const id = groupIdOf(g);
    if (next.has(id)) next.delete(id); else next.add(id);
    return next;
  });
  return (
    <div style={{ padding: 0, display: "flex", flexDirection: "column", flex: 1, minHeight: 0 }}>
      <Label style={{ color: "var(--text-faint)", marginBottom: 10, flexShrink: 0 }}>Right-click a component for more actions, including uncheck and prompt tools.</Label>
      <div style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 4,
        position: "relative",
        zIndex: 1,
      }}>
        {tabs.map((t) => (
          <GameTab key={t.id} active={gameTab === t.id} onClick={() => setGameTab(t.id)}>{t.label}</GameTab>
        ))}
        <div style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          gap: 8,
          paddingLeft: 12,
          paddingRight: 4,
          height: 30,
          boxSizing: "border-box",
          flexWrap: "wrap",
        }}>
          {conflictCount > 0 && <Pill tone="danger">{conflictCount} conflict{conflictCount > 1 ? "s" : ""}</Pill>}
          {promptCount > 0 && <Pill tone="warn">{promptCount} prompt{promptCount > 1 ? "s" : ""}</Pill>}
          <span style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
            <TopButton>Undo</TopButton>
            <TopButton>Redo</TopButton>
            <TopButton>Collapse All</TopButton>
            <TopButton>Expand All</TopButton>
          </span>
        </div>
      </div>
      <Box style={{ padding: 10, flex: 1, minHeight: 0, overflow: "auto" }}>
        {byMod.map((g, gi) => {
          const groupKeys = g.items.map((c) => keyOf(c));
          const groupDragging = dragKeys.length > 0 && groupKeys.every((k) => dragKeys.includes(k));
          const headerDropAbove = dragOver && dragOver.idx === g.items[0]._idx && dragOver.side === "above";
          const gapDropOver = (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
            if (!dragOver || dragOver.idx !== g.items[0]._idx || dragOver.side !== "above") {
              setDragOver({ idx: g.items[0]._idx, side: "above" });
            }
          };
          return (
          <React.Fragment key={`${g.tp2}-${gi}`}>
            {gi > 0 && (
              <div
                onDragOver={gapDropOver}
                onDrop={handleDrop}
                style={{ height: 12, display: "flex", alignItems: "center" }}
              >
                {headerDropAbove && (
                  <div style={{ height: 2, background: "var(--accent)", borderRadius: 1, flex: 1, margin: "0 4px" }} />
                )}
              </div>
            )}
            {gi === 0 && headerDropAbove && dropLine}
            <div
              draggable
              onDragStart={(e) => {
                e.dataTransfer.effectAllowed = "move";
                setDragKeys(groupKeys);
                setSelectedIds(new Set(groupKeys));
                setAnchorIdx(g.items[0]._idx);
                e.dataTransfer.setData("text/plain", JSON.stringify(groupKeys));
              }}
              onDragOver={gapDropOver}
              onDrop={handleDrop}
              onDragEnd={handleDragEnd}
              title="Drag to move this mod's components together"
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "4px 6px",
                background: "var(--rail-bg)",
                border: "1px dashed #b9b09a",
                fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                fontSize: 13,
                fontWeight: 500,
                cursor: "grab",
                userSelect: "none",
                opacity: groupDragging ? 0.4 : 1,
              }}
            >
              <span onClick={(e) => e.stopPropagation()} style={{ cursor: "pointer", userSelect: "none" }}>🔒</span>
              <span
                onClick={(e) => { e.stopPropagation(); toggleGroupCollapsed(g); }}
                title={isGroupCollapsed(g) ? "Expand" : "Collapse"}
                style={{ cursor: "pointer", userSelect: "none", padding: "2px 4px", margin: "-2px -4px" }}
              >🔗 {isGroupCollapsed(g) ? "▸" : "▾"}</span>
              <span>{g.modName}{g.copySuffix && <span style={{ color: "var(--text-faint)", fontWeight: 400 }}>{g.copySuffix}</span>}</span>
              <span style={{ color: "var(--text-faint)", fontWeight: 400 }}>({g.items.length})</span>
              <span style={{ color: "var(--text-faint)", fontWeight: 400, fontSize: 12 }}>v{g.version}</span>
            </div>
            {!isGroupCollapsed(g) && g.items.map((c, ii) => {
              const isSel = selectedIds.has(keyOf(c));
              const dragging = dragKeys.includes(keyOf(c));
              const isFirstInGroup = ii === 0;
              const showAbove = dragOver && dragOver.idx === c._idx && dragOver.side === "above" && !isFirstInGroup;
              const showBelow = dragOver && dragOver.idx === c._idx && dragOver.side === "below";
              return (
              <React.Fragment key={`${c.tp2}-${c.id}`}>
                {showAbove && dropLine}
                <div
                  className={`tree-row${isSel ? " is-selected" : ""}`}
                  draggable
                  onDragStart={handleDragStart(c)}
                  onDragOver={handleDragOver(c._idx)}
                  onDrop={handleDrop}
                  onDragEnd={handleDragEnd}
                  onClick={(e) => handleRowClick(c, e)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    paddingLeft: 18,
                    paddingRight: 6,
                    paddingTop: 2,
                    paddingBottom: 2,
                    borderBottom: "1px dashed var(--border-dashed-light)",
                    cursor: "pointer",
                    userSelect: "none",
                    opacity: dragging ? 0.4 : 1,
                  }}
                >
                  <span style={{ color: "var(--text-faint)", cursor: "grab", fontSize: 16, lineHeight: 1, userSelect: "none" }}>≡</span>
                  <span style={{
                    color: "var(--text-faint)",
                    fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                    fontSize: 11,
                    minWidth: String(items.length).length * 8 + 6,
                    textAlign: "right",
                  }}>{c._idx + 1}.</span>
                  <WeiduLine c={c} style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis" }} />
                  {c.conflict && <Pill tone="danger">Conflict</Pill>}
                  {c.prompt && <Pill tone="warn">Prompt</Pill>}
                </div>
                {showBelow && dropLine}
              </React.Fragment>
              );
            })}
          </React.Fragment>
          );
        })}
      </Box>
    </div>
  );
};

const OrderPanel = ({ gameTab, setGameTab, game = "EET", orderByTab }) => {
  const tabs = tabsForGame(game);
  const selected = (orderByTab && orderByTab[gameTab]) || [];
  const upperTab = (tabs.find((t) => t.id === gameTab) || tabs[0]).label;
  return (
    <div style={{ padding: 0, display: "flex", flexDirection: "column", flex: 1, minHeight: 0 }}>
      <div style={{ display: "flex", gap: 8, marginBottom: 10, alignItems: "center", flexShrink: 0 }}>
        <Btn>Save weidu.log's</Btn>
        <Label hand style={{ marginLeft: "auto", color: "var(--text-faint)" }}>
          {selected.length} components ready to install on {upperTab} · across {new Set(selected.map((c) => c.tp2)).size} mods
        </Label>
      </div>
      <div style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 4,
        position: "relative",
        zIndex: 1,
      }}>
        {tabs.map((t) => (
          <GameTab key={t.id} active={gameTab === t.id} onClick={() => setGameTab(t.id)}>{t.label}</GameTab>
        ))}
      </div>
      <Box style={{ padding: 12, flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
        <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
          {selected.length === 0 ? (
            <Label style={{ color: "var(--text-faint)" }}>No components selected on {upperTab}.</Label>
          ) : (
            (() => {
              const lineNumWidth = String(selected.length).length * 9 + 4;
              return selected.map((c, i) => (
                <div key={`${c.tp2}-${c.id}`} style={{ display: "flex", alignItems: "baseline", gap: 10 }}>
                  <span style={{
                    color: "var(--text-faint)",
                    fontFamily: "'FiraCode Nerd', monospace",
                    fontSize: 12,
                    minWidth: lineNumWidth,
                    textAlign: "right",
                    flexShrink: 0,
                    userSelect: "none",
                  }}>{i + 1}</span>
                  <WeiduLine c={c} style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis" }} />
                </div>
              ));
            })()
          )}
        </div>
      </Box>
    </div>
  );
};

const FinalPlanPanel = ({ installComplete = false, setInstallComplete = () => {} }) => (
  <div style={{ padding: 0, display: "flex", flexDirection: "column", flex: 1, minHeight: 0 }}>
    {installComplete ? (
      <Box style={{ padding: "10px 14px", marginBottom: 10, display: "flex", alignItems: "center", gap: 12, flexShrink: 0, borderColor: "var(--success)" }}>
        <Pill tone="info" style={{ background: "var(--success)", color: "#0B1116" }}>Installed</Pill>
        <Label>9 mods · 136 components · no errors</Label>
        <Label hand style={{ marginLeft: "auto", color: "var(--text-faint)" }}>ran 4:12 · finished a few seconds ago</Label>
      </Box>
    ) : (
      <Label green style={{ color: "var(--success)", marginBottom: 10, flexShrink: 0 }}>Dev Mode: RUST_LOG=DEBUG selected.</Label>
    )}
    <div style={{ display: "grid", gridTemplateColumns: "1.2fr 1fr", gap: 10, marginBottom: 10, flexShrink: 0 }}>
      <Box label="Command" style={{ padding: 10 }}>
        <Mono>"D:\Modding\Installer\WeiDU-Windows\mod_runner.exe" eet</Mono>
        <Mono>--bg1-game-directory "D:\import test\Baldur's Gate Enhanced Edition"</Mono>
        <Mono>--bg1-log-file "D:\import test\logs\bg1\weidu.log"</Mono>
        <Mono>--bg2-game-directory "D:\import test\Baldur's Gate II Enhanced Edition"</Mono>
        <Mono>--mod-directories "D:\import test\Mods"</Mono>
      </Box>
      <Box label="Summary" style={{ padding: 10 }}>
        <div style={{ display: "grid", gridTemplateColumns: "130px 1fr", gap: "6px 12px" }}>
          <Label>Game Install:</Label><Label>EET</Label>
          <Label>Mods Folder:</Label><Label>D:\import test\Mods</Label>
          <Label>WeiDU binary:</Label><Label>D:\Modding\Installer\weidu.exe</Label>
          <Label>Language:</Label><Label>en_US</Label>
          <Label>BGEE log:</Label><Label>D:\import test\logs\test\bg1\weidu.log</Label>
        </div>
      </Box>
    </div>
    <div style={{ display: "flex", gap: 8, marginBottom: 8, flexShrink: 0, flexWrap: "wrap", alignItems: "center" }}>
      {installComplete ? (
        <TopButton disabled>✓ Installed</TopButton>
      ) : (
        <TopButton primary onClick={() => setInstallComplete(true)}>Install</TopButton>
      )}
      <TopButton>Actions</TopButton>
      <TopButton>Diagnostics</TopButton>
      <TopButton>Prompt Answers</TopButton>
      <Label>☑ General</Label>
      <Label>☐ Important Only</Label>
      <Label>☐ Installed Only</Label>
      <Label>☑ Auto-scroll</Label>
    </div>
    <Box label="Console" style={{ padding: 10, flex: 1, minHeight: 0, overflow: "auto" }}>
      {installComplete ? (
        <>
          <Mono>[install] starting mod_installer · 9 mods · 136 components</Mono>
          <Mono green>SUCCESSFULLY INSTALLED Merge "Siege of Dragonspear" DLC: 1.8</Mono>
          <Mono green>SUCCESSFULLY INSTALLED Core Fixes: Beta 2</Mono>
          <Mono green>SUCCESSFULLY INSTALLED Game Text Update: Beta 2</Mono>
          <Mono muted>… 133 more components …</Mono>
          <Mono green>SUCCESSFULLY INSTALLED Smarter mages: 35.21</Mono>
          <Mono green>[install] all 136 components installed in 4m 12s · 0 errors · 0 warnings</Mono>
        </>
      ) : (
        <Mono muted>Console output appears here while mod_installer runs...</Mono>
      )}
    </Box>
  </div>
);

const InstallPanel = FinalPlanPanel;

const WORKSPACE_STEPS = [
  { id: "sources", step: "Step 2", label: "Scan and Select", hint: "Choose components to install." },
  { id: "components", step: "Step 3", label: "Reorder and Resolve", hint: "Review and adjust install order. Drag to reorder; right-click for more actions." },
  { id: "order", step: "Step 4", label: "Review", hint: "Verify setup and install order before running. Next saves weidu.log file(s) and advances to install." },
  { id: "final", step: "Step 5", label: "Install", hint: "Run the install with live console, prompts, and diagnostics." },
];

const WorkspaceProgressBar = ({ active, completed }) => {
  const activeIdx = WORKSPACE_STEPS.findIndex((s) => s.id === active);
  return (
    <div style={{
      display: "flex",
      ...sketchyBorder,
      boxShadow: "3px 3px 0 var(--shadow)",
      overflow: "hidden",
      marginBottom: 8,
    }}>
      {WORKSPACE_STEPS.map((s, i) => {
        const isCurrent = i === activeIdx;
        const isCompleted = !isCurrent && completed.includes(s.id);
        const isUpcoming = !isCurrent && !isCompleted;
        return (
          <div
            key={s.id}
            style={{
              flex: 1,
              padding: "5px 12px",
              borderRight: i < WORKSPACE_STEPS.length - 1 ? "1.5px solid var(--border-strong)" : "none",
              background: isCurrent ? "var(--accent)" : isUpcoming ? "var(--chrome-bg)" : "var(--shell-bg)",
              opacity: isUpcoming ? 0.55 : 1,
              display: "flex",
              alignItems: "center",
              gap: 10,
              fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
              userSelect: "none",
              minHeight: 30,
            }}
          >
            <span style={{
              fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
              fontSize: 10,
              fontWeight: 500,
              letterSpacing: 1.4,
              textTransform: "uppercase",
              color: isUpcoming ? "var(--text-faint)" : isCompleted ? "var(--text-muted)" : isCurrent ? "#1a2638" : "var(--text)",
              flexShrink: 0,
            }}>
              {s.step}
            </span>
            <span style={{
              fontSize: isCurrent ? 14 : 13,
              fontWeight: isCurrent ? 700 : 400,
              lineHeight: 1,
              color: isUpcoming ? "var(--text-faint)" : isCurrent ? "#1a2638" : "var(--text)",
            }}>
              {s.label}
            </span>
            {isCompleted && (
              <span style={{ marginLeft: "auto", color: "var(--success)", fontSize: 15, lineHeight: 1 }}>✓</span>
            )}
          </div>
        );
      })}
    </div>
  );
};

const WorkspaceNavBar = ({ currentIdx, total, currentLabel, onPrev, onNext, disablePrev = false }) => {
  const isFirst = currentIdx === 0;
  const isLast = currentIdx === total - 1;
  return (
    <div style={{
      marginTop: 20,
      paddingTop: 14,
      borderTop: "1.5px dashed var(--border-soft)",
      display: "flex",
      alignItems: "center",
      gap: 12,
    }}>
      <Btn
        small
        disabled={disablePrev}
        onClick={onPrev}
        title={disablePrev ? "Disabled while install is running or after a successful install" : undefined}
      >← Previous</Btn>
      <Label hand style={{ color: "var(--text-faint)", marginLeft: 14 }}>
        on {currentLabel} · step {currentIdx + 1} of {total}
      </Label>
      <div style={{ marginLeft: "auto", display: "flex", gap: 10, alignItems: "center" }}>
        <Label hand style={{ color: "var(--text-faint)" }}>
          {isLast ? "final step" : `next: ${WORKSPACE_STEPS[currentIdx + 1].label}`}
        </Label>
        <Btn primary disabled={isLast} onClick={onNext}>Next →</Btn>
      </div>
    </div>
  );
};

const WorkspaceView = ({ source, initialTab = "sources", fork, game = "EET", modlistName = "Untitled modlist" }) => {
  const [tab, setTab] = useState(initialTab);
  const [completed, setCompleted] = useState([]);
  const initialGameTab = tabsForGame(game)[0].id;
  const [gameTab, setGameTab] = useState(initialGameTab);
  // Inline rename: ✎ icon next to "Editing X" toggles edit mode. Wireframe state only.
  const [displayName, setDisplayName] = useState(modlistName);
  const [renamingName, setRenamingName] = useState(false);
  const [tempName, setTempName] = useState(displayName);
  const startRename = () => { setTempName(displayName); setRenamingName(true); };
  const saveRename = () => { if (tempName.trim()) setDisplayName(tempName.trim()); setRenamingName(false); };
  const cancelRename = () => setRenamingName(false);
  const [draftSavedAt, setDraftSavedAt] = useState(0); // timestamp; non-zero means "show saved! affordance"
  const onSaveDraft = () => {
    // Wireframe: persist this build into the in-progress registry. The MOCK_IN_PROGRESS_BUILDS
    // would gain (or update) an entry for the current modlist. Confirmation is inline.
    setDraftSavedAt(Date.now());
    setTimeout(() => setDraftSavedAt(0), 1600);
  };
  const [sharePasteOpen, setSharePasteOpen] = useState(false);
  const [forkInfoOpen, setForkInfoOpen] = useState(false);
  const [installComplete, setInstallComplete] = useState(false);
  // Lifted state: ordered array per game tab. Each item is "checked"; un-checking removes it,
  // re-checking appends to end. Order is the source of truth for Steps 2–4.
  const [orderByTab, setOrderByTab] = useState(() => {
    const out = {};
    for (const gt of Object.keys(MOCK_MODS_BY_TAB)) out[gt] = collectSelectedInOrder(gt);
    return out;
  });
  const currentIdx = WORKSPACE_STEPS.findIndex((s) => s.id === tab);
  const current = WORKSPACE_STEPS[currentIdx];

  const goNext = () => {
    if (currentIdx >= WORKSPACE_STEPS.length - 1) return;
    if (!completed.includes(tab)) setCompleted([...completed, tab]);
    setTab(WORKSPACE_STEPS[currentIdx + 1].id);
  };
  const goPrev = () => {
    if (currentIdx === 0) return;
    setTab(WORKSPACE_STEPS[currentIdx - 1].id);
  };

  return (
    <div
      className="sk-page"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        minHeight: 0,
        padding: "20px 28px",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-end", marginBottom: 10, flexShrink: 0, gap: 12 }}>
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
            {renamingName ? (
              <>
                <span style={{
                  fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                  fontSize: 13,
                  fontWeight: 500,
                  color: "var(--text)",
                  lineHeight: 1,
                }}>Editing</span>
                <input
                  autoFocus
                  value={tempName}
                  onChange={(e) => setTempName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveRename();
                    if (e.key === "Escape") cancelRename();
                  }}
                  style={{
                    ...sketchyBorder,
                    fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                    fontSize: 13,
                    fontWeight: 500,
                    color: "var(--text)",
                    background: "var(--input-bg)",
                    padding: "4px 8px",
                    width: 240,
                    outline: "none",
                  }}
                />
                <Btn small primary onClick={saveRename}>save</Btn>
                <Btn small onClick={cancelRename}>cancel</Btn>
              </>
            ) : (
              <>
                <h1 style={{
                  fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                  fontSize: 13,
                  margin: 0,
                  fontWeight: 500,
                  color: "var(--text)",
                  lineHeight: 1,
                }}>
                  Editing {displayName}
                </h1>
                <span
                  onClick={startRename}
                  title="Rename modlist"
                  style={{
                    cursor: "pointer",
                    color: "var(--text-muted)",
                    fontSize: 13,
                    lineHeight: 1,
                    padding: "2px 4px",
                    userSelect: "none",
                  }}
                >✎</span>
              </>
            )}
            {fork && (
              <div style={{
                ...sketchyBorder,
                background: "var(--accent)",
                boxShadow: "2px 2px 0 var(--shadow)",
                padding: "4px 12px",
                fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                fontSize: 10,
                fontWeight: 500,
                letterSpacing: 1.5,
                textTransform: "uppercase",
                alignSelf: "center",
                whiteSpace: "nowrap",
              }}>
                ⑂ Fork
              </div>
            )}
          </div>
          <div style={{ fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif", fontSize: 16, color: "var(--text-muted)", marginTop: 4 }}>
            {fork ? (
              <span>
                <span style={{ color: "var(--accent-deep)", fontWeight: 700 }}>⑂ Forked from</span>{" "}
                <strong style={{ color: "var(--text)" }}>"{fork.name}"</strong>
                {fork.author && <span> by {fork.author}</span>}
                {" · "}
                {fork.mods} mods · {fork.components} components preselected
              </span>
            ) : (
              `${source} · shared BIO workflow`
            )}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
          {fork && <Btn small onClick={() => setForkInfoOpen(true)}>⑂ view fork details</Btn>}
          {tab === "final" ? (
            <Btn
              small
              primary={installComplete}
              disabled={!installComplete}
              onClick={() => installComplete && setSharePasteOpen(true)}
              title={installComplete ? "View and copy the import code for this modlist" : "Available after a successful install"}
            >Share import code</Btn>
          ) : (
            <Btn
              small
              onClick={onSaveDraft}
              title="Save this in-progress build so you can resume it from Home"
            >{draftSavedAt ? "✓ saved!" : "save draft"}</Btn>
          )}
        </div>
      </div>
      <div style={{ flexShrink: 0 }}>
        <WorkspaceProgressBar active={tab} completed={completed} />
      </div>
      <div style={{ marginBottom: 10, marginLeft: 4, flexShrink: 0 }}>
        <Label hand style={{ color: "var(--text-faint)", fontSize: 14 }}>{current.hint}</Label>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {tab === "sources" && <SourcesPanel gameTab={gameTab} setGameTab={setGameTab} fork={fork} game={game} orderByTab={orderByTab} setOrderByTab={setOrderByTab} />}
        {tab === "components" && <ComponentsPanel gameTab={gameTab} setGameTab={setGameTab} game={game} orderByTab={orderByTab} setOrderByTab={setOrderByTab} />}
        {tab === "order" && <OrderPanel gameTab={gameTab} setGameTab={setGameTab} game={game} orderByTab={orderByTab} />}
        {tab === "final" && <FinalPlanPanel installComplete={installComplete} setInstallComplete={setInstallComplete} />}
      </div>
      <div style={{ flexShrink: 0 }}>
        <WorkspaceNavBar
          currentIdx={currentIdx}
          total={WORKSPACE_STEPS.length}
          currentLabel={`${current.step} · ${current.label}`}
          onPrev={goPrev}
          onNext={goNext}
          disablePrev={tab === "final" && installComplete}
        />
      </div>
      <SharePasteCodeDialog
        open={sharePasteOpen}
        onClose={() => setSharePasteOpen(false)}
      />
      <ForkInfoPopup
        open={forkInfoOpen}
        onClose={() => setForkInfoOpen(false)}
        lineage={(fork && fork.forkedFrom) || []}
        self={{ name: displayName, author: "@you" }}
      />
    </div>
  );
};

// ---------- FORK SUB-FLOW ----------
const FORK_MOD_LIST = [
  { name: "DLCMerger", status: "done", source: "argent77" },
  { name: "EEFixPack", status: "done", source: "gibberlings3" },
  { name: "BG1UB", status: "done", source: "pocket-plane-group" },
  { name: "Tweaks Anthology (cdtweaks)", status: "extracting", source: "gibberlings3" },
  { name: "EET", status: "downloading", source: "gibberlings3", progress: 62 },
  { name: "EEex", status: "queued", source: "bubb" },
  { name: "LEUI", status: "queued", source: "lefrefut" },
  { name: "EEUITweaks", status: "queued", source: "r-e-d" },
  { name: "Sword Coast Stratagems", status: "queued", source: "gibberlings3" },
];

const FORK_META = {
  name: "Born2BSalty's EET tactical playthrough",
  author: "@b2bs",
  game: "EET",
  mods: 9,
  components: 136,
  bgeeEntries: 21,
  bg2eeEntries: 115,
  // Lineage, oldest → newest ancestors (SPEC §13.3 `forked_from`). This
  // modlist's own name/author are the fields above; forkedFrom is the chain
  // it descends from. Append-only — the original creator stays first.
  forkedFrom: [
    { name: "EET Basics", author: "@olim" },
    { name: "EET Tactical", author: "@b2bs" },
  ],
};

const SubFlowFooter = ({ onBack, backLabel = "Back", onPrimary, primaryLabel, hint }) => (
  <div style={{
    marginTop: 20,
    paddingTop: 14,
    borderTop: "1.5px dashed var(--border-soft)",
    display: "flex",
    alignItems: "center",
    gap: 12,
    flexShrink: 0,
  }}>
    {onBack && <Btn small onClick={onBack}>← {backLabel}</Btn>}
    {hint && <Label hand style={{ color: "var(--text-faint)", marginLeft: 6 }}>{hint}</Label>}
    <div style={{ marginLeft: "auto" }}>
      <Btn primary onClick={onPrimary}>{primaryLabel}</Btn>
    </div>
  </div>
);

const ForkPasteScreen = ({ onBack, onPreview }) => (
  <div className="sk-page" style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, padding: "20px 28px" }}>
    <div style={{ marginBottom: 12, flexShrink: 0 }}>
      <ScreenTitle title="Fork existing modlist" sub="paste a BIO share code, then preview before downloading" />
    </div>
    <Box label="import code" style={{ padding: "20px", flexShrink: 0 }}>
      <Label style={{ marginBottom: "8px" }}>BIO-MODLIST-V1 share code</Label>
      <div style={{
        ...sketchyBorder,
        minHeight: 230,
        padding: "12px",
        background: "var(--input-bg)",
        fontFamily: "'FiraCode Nerd', monospace",
        fontSize: 12,
        color: "var(--text-faint)",
        whiteSpace: "pre-wrap",
      }}>
        BIO-MODLIST-V1:eJyrVkrLz1eyUkpKLFKqBQA...{"\n\n"}Paste the full code here.
      </div>
    </Box>
    <div style={{ flex: 1 }} />
    <SubFlowFooter
      onBack={onBack}
      onPrimary={onPreview}
      primaryLabel="Preview →"
      hint="no download starts until preview is accepted"
    />
  </div>
);

const ForkPreviewScreen = ({ onBack, onAccept }) => {
  const [tab, setTab] = useState("Summary");
  const [forkInfoOpen, setForkInfoOpen] = useState(false);
  return (
    <div className="sk-page" style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, padding: "20px 28px" }}>
      <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12 }}>
        <ScreenTitle title={FORK_META.name} sub={`by ${FORK_META.author} · review then fork into a new workspace`} />
        {FORK_META.forkedFrom && FORK_META.forkedFrom.length > 0 && (
          <div style={{ flexShrink: 0, marginTop: 4 }}>
            <Btn small onClick={() => setForkInfoOpen(true)}>⑂ fork info</Btn>
          </div>
        )}
      </div>
      <Box style={{ padding: "14px", flexShrink: 0, marginBottom: 12 }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "6px 16px", fontSize: 14 }}>
          <Label>Game: <strong>{FORK_META.game}</strong></Label>
          <Label>Mods: <strong>{FORK_META.mods}</strong></Label>
          <Label>Components: <strong>{FORK_META.components}</strong></Label>
          <Label>BGEE/BG2EE entries: <strong>{FORK_META.bgeeEntries}/{FORK_META.bg2eeEntries}</strong></Label>
        </div>
      </Box>
      <ImportPreviewTabs active={tab} setActive={setTab} merge />
      <Box style={{ padding: "14px", flex: 1, minHeight: 0, overflow: "auto" }}>
        <PreviewText tab={tab} />
      </Box>
      <SubFlowFooter
        onBack={onBack}
        onPrimary={onAccept}
        primaryLabel="Begin Import →"
        hint="downloads mods, applies selection + order, then drops you on Step 2"
      />
      <ForkInfoPopup
        open={forkInfoOpen}
        onClose={() => setForkInfoOpen(false)}
        lineage={FORK_META.forkedFrom}
        self={{ name: FORK_META.name, author: FORK_META.author }}
      />
    </div>
  );
};

const ImportDownloadScreen = ({ title, sub, onCancel, onContinue, hint, continueLabel = "simulate complete →" }) => {
  const statusColor = (s) => s === "done" ? "var(--success)" : s === "queued" ? "var(--text-faint)" : "var(--text)";
  const statusText = (m) => m.status === "done" ? "✓ staged"
    : m.status === "extracting" ? "extracting..."
    : m.status === "downloading" ? `downloading ${m.progress}%`
    : "queued";
  const barPct = (m) => m.status === "done" ? "100%"
    : m.status === "extracting" ? "80%"
    : m.status === "downloading" ? `${m.progress}%`
    : "0%";
  const overall = 28;
  return (
    <div className="sk-page" style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, padding: "20px 28px" }}>
      <ScreenTitle title={title} sub={sub} />
      <Box label="overall progress" style={{ padding: "14px", marginBottom: 14, flexShrink: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <Label style={{ fontSize: 16, width: 180, flexShrink: 0 }}>3 / 9 mods · {overall}%</Label>
          <div style={{ flex: 1, height: 14, ...sketchyBorder, background: "var(--input-bg)", overflow: "hidden" }}>
            <div style={{ width: `${overall}%`, height: "100%", background: "var(--accent)" }} />
          </div>
        </div>
        {hint && (
          <Label hand style={{ marginTop: 6, color: "var(--text-faint)", fontSize: 14 }}>{hint}</Label>
        )}
      </Box>
      <Box label="mod progress" style={{ padding: "12px", minHeight: 360 }}>
        <div style={{ display: "grid", gridTemplateColumns: "1.8fr 1fr 130px 120px", gap: "6px 12px", alignItems: "center", fontSize: 14 }}>
          <Label hand style={{ color: "var(--text-muted)" }}>mod</Label>
          <Label hand style={{ color: "var(--text-muted)" }}>source</Label>
          <Label hand style={{ color: "var(--text-muted)" }}>status</Label>
          <Label hand style={{ color: "var(--text-muted)" }}>progress</Label>
          {FORK_MOD_LIST.map((m) => (
            <React.Fragment key={m.name}>
              <Label style={{ color: statusColor(m.status) === "var(--text-faint)" ? "var(--text-faint)" : "var(--text)" }}>{m.name}</Label>
              <Label style={{ fontSize: 13, color: "var(--text-faint)" }}>{m.source}</Label>
              <Label style={{ color: statusColor(m.status) }}>{statusText(m)}</Label>
              <div style={{ height: 8, ...sketchyBorder, background: "var(--input-bg)", overflow: "hidden" }}>
                <div style={{
                  width: barPct(m),
                  height: "100%",
                  background: m.status === "queued" ? "transparent" : "var(--accent)",
                }} />
              </div>
            </React.Fragment>
          ))}
        </div>
      </Box>
      <div style={{ flex: 1 }} />
      <SubFlowFooter
        onBack={onCancel}
        backLabel="Cancel"
        onPrimary={onContinue}
        primaryLabel={continueLabel}
      />
    </div>
  );
};

const CreateScreen = ({ resumedBuild, resumeBuild }) => {
  // states: choose | scratch | fork-paste | fork-preview | fork-download | fork-workspace
  const [mode, setMode] = useState("choose");
  const [dest, setDest] = useState("");
  const [destChoice, setDestChoice] = useState(null);
  const [game, setGame] = useState("EET");
  const [loadDraftOpen, setLoadDraftOpen] = useState(false);
  const [modlistName, setModlistName] = useState("");

  const handleBrowse = () => {
    setDest("D:\\BG2EE_install_test");
    setDestChoice(null);
  };

  // Resumed in-progress build (via Home Resume or Load Draft dialog) → workspace opens directly at Step 2
  // with the build's name/game/source. Wireframe demo: workspace mock data is shared, so this is largely a label change.
  if (resumedBuild) {
    return (
      <WorkspaceView
        source={`Resumed "${resumedBuild.n}"`}
        game={resumedBuild.game || "EET"}
        modlistName={resumedBuild.n}
        initialTab="sources"
      />
    );
  }

  if (mode === "scratch") {
    return <WorkspaceView source="From local install" game={game} modlistName={modlistName || "Untitled modlist"} />;
  }
  if (mode === "fork-paste") {
    return <ForkPasteScreen onBack={() => setMode("choose")} onPreview={() => setMode("fork-preview")} />;
  }
  if (mode === "fork-preview") {
    return <ForkPreviewScreen onBack={() => setMode("fork-paste")} onAccept={() => setMode("fork-download")} />;
  }
  if (mode === "fork-download") {
    return (
      <ImportDownloadScreen
        title="Downloading fork"
        sub="fetching mod archives and extracting into the staging folder"
        hint="after download: components auto-selected · order applied · lands on Step 2 for review"
        onCancel={() => setMode("fork-preview")}
        onContinue={() => setMode("fork-workspace")}
        continueLabel="continue to Step 2 →"
      />
    );
  }
  if (mode === "fork-workspace") {
    return <WorkspaceView source={`Forked from “${FORK_META.name}”`} fork={FORK_META} modlistName={FORK_META.name} />;
  }

  return (
    <div className="sk-page">
      <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12 }}>
        <ScreenTitle title="Create / edit modlist" sub="name your modlist, set destination + mods paths, then pick a starting point" />
        <Btn small onClick={() => setLoadDraftOpen(true)}>load draft</Btn>
      </div>

      <Box style={{ padding: "16px 20px", marginBottom: 18 }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: 14 }}>
          <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 16, alignItems: "end" }}>
            <div>
              <Label hand style={{ marginBottom: 4, color: "var(--text-muted)" }}>modlist name</Label>
              <input
                value={modlistName}
                onChange={(e) => setModlistName(e.target.value)}
                placeholder="e.g. Tactical EET 2026"
                style={{
                  ...sketchyBorder,
                  width: "100%",
                  background: "var(--input-bg)",
                  fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                  fontSize: 14,
                  padding: "8px 12px",
                  color: "var(--text)",
                  outline: "none",
                  boxSizing: "border-box",
                }}
              />
            </div>
            <div>
              <Label hand style={{ marginBottom: 4, color: "var(--text-muted)" }}>game</Label>
              <select
                value={game}
                onChange={(e) => setGame(e.target.value)}
                style={{
                  ...sketchyBorder,
                  padding: "8px 12px",
                  background: "var(--input-bg)",
                  fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                  fontSize: 14,
                  color: "var(--text)",
                  cursor: "pointer",
                  outline: "none",
                }}
              >
                {["EET", "BGEE", "BG2EE", "IWDEE"].map((g) => <option key={g} value={g}>{g}</option>)}
              </select>
            </div>
          </div>
          <FolderInput
            label="destination folder"
            placeholder="D:\BG2EE_install_test"
            value={dest}
            onBrowse={handleBrowse}
          />
          {dest && <DestinationNotEmptyWarning choice={destChoice} setChoice={setDestChoice} allowPartial={false} />}
        </div>
      </Box>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
        <Box
          style={{ padding: "20px 22px", cursor: "pointer", display: "flex", flexDirection: "column" }}
          onClick={() => setMode("scratch")}
        >
          <Label hand style={{ fontSize: "18px", marginBottom: 8, lineHeight: 1.2 }}>
            New modlist from downloaded mods
          </Label>
          <Label style={{ color: "var(--text-muted)", marginBottom: 16, flex: 1 }}>
            Scan your local mods folder, pick components, reorder, then install. Starts from an empty selection.
          </Label>
          <Btn primary style={{ alignSelf: "flex-start" }}>start →</Btn>
        </Box>
        <Box
          style={{ padding: "20px 22px", cursor: "pointer", display: "flex", flexDirection: "column" }}
          onClick={() => setMode("fork-paste")}
        >
          <Label hand style={{ fontSize: "18px", marginBottom: 8, lineHeight: 1.2 }}>
            Import and modify another modlist
          </Label>
          <Label style={{ color: "var(--text-muted)", marginBottom: 16, flex: 1 }}>
            Paste a share code. BIO downloads the mods, preselects components, applies the order, then drops you on Step 2 to review and adjust.
          </Label>
          <Btn primary style={{ alignSelf: "flex-start" }}>paste share code →</Btn>
        </Box>
      </div>
      <LoadDraftDialog
        open={loadDraftOpen}
        onCancel={() => setLoadDraftOpen(false)}
        onResume={(build) => { setLoadDraftOpen(false); resumeBuild && resumeBuild(build); }}
      />
    </div>
  );
};

// ---------- SETTINGS ----------
const Check = ({ on }) => (
  <div
    style={{
      ...sketchyBorder,
      width: 18, height: 18,
      display: "flex", alignItems: "center", justifyContent: "center",
      background: on ? "var(--accent)" : "var(--input-bg)",
      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
      fontSize: 16, lineHeight: 1, fontWeight: 500,
      flexShrink: 0,
    }}
  >
    {on ? "✓" : ""}
  </div>
);

const ValueRow = ({ label, placeholder, hint }) => (
  <div style={{ display: "flex", alignItems: "center", padding: "4px 0", gap: 10, borderBottom: "1px dashed var(--border-dashed-light)" }}>
    <Label style={{ flex: 1, fontSize: 14 }}>{label}</Label>
    {hint && <Label hand style={{ fontSize: 12, color: "var(--text-faint)" }}>{hint}</Label>}
    <Input mono placeholder={placeholder} style={{ width: 90, padding: "2px 6px", fontSize: 11, textAlign: "right" }} />
  </div>
);

const ToggleRow = ({ label, on, hint }) => (
  <div style={{ display: "flex", alignItems: "center", padding: "4px 0", gap: 10, borderBottom: "1px dashed var(--border-dashed-light)" }}>
    <Label style={{ flex: 1, fontSize: 14 }}>{label}</Label>
    {hint && <Label hand style={{ fontSize: 12, color: "var(--text-faint)" }}>{hint}</Label>}
    <Toggle on={on} />
  </div>
);

const CheckRow = ({ on, label, value, hint }) => (
  <div style={{ display: "flex", alignItems: "center", padding: "4px 0", gap: 10, borderBottom: "1px dashed var(--border-dashed-light)" }}>
    <Check on={on} />
    <Label style={{ flex: 1, fontSize: 14 }}>{label}</Label>
    {hint && <Label hand style={{ fontSize: 12, color: "var(--text-faint)" }}>{hint}</Label>}
    {value !== undefined && (
      <Input mono value={String(value)} style={{ width: 70, padding: "2px 6px", fontSize: 11, textAlign: "right" }} />
    )}
  </div>
);

const SectionHead = ({ children }) => (
  <Label hand style={{ fontSize: 16, color: "var(--accent-deep)", marginTop: 8, marginBottom: 2, letterSpacing: 0.5 }}>
    · {children}
  </Label>
);

const SettingsRow = ({ label, control, hint }) => (
  <div style={{ display: "flex", alignItems: "center", padding: "10px 0", borderBottom: "1px dashed #ccc", gap: "16px" }}>
    <div style={{ flex: 1 }}>
      <Label>{label}</Label>
      {hint && <Label hand style={{ fontSize: "13px", color: "var(--text-faint)" }}>{hint}</Label>}
    </div>
    <div style={{ flexShrink: 0 }}>{control}</div>
  </div>
);

const NameRow = ({ name, setName }) => {
  const [editing, setEditing] = useState(false);
  const [temp, setTemp] = useState(name);
  React.useEffect(() => { if (!editing) setTemp(name); }, [name, editing]);
  const inputStyle = {
    ...sketchyBorder,
    padding: "4px 10px",
    background: "var(--input-bg)",
    fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
    fontSize: 14,
    color: "var(--text)",
    width: 200,
    outline: "none",
  };
  return (
    <SettingsRow
      label="Your name"
      hint="credited as the author on any modlists you create or share"
      control={editing ? (
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input
            value={temp}
            onChange={(e) => setTemp(e.target.value)}
            placeholder="@yourhandle"
            style={inputStyle}
          />
          <Btn small primary onClick={() => { setName(temp); setEditing(false); }}>save</Btn>
          <Btn small onClick={() => { setTemp(name); setEditing(false); }}>cancel</Btn>
        </div>
      ) : (
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <Label style={{ fontSize: 15, color: name ? "var(--text)" : "var(--text-faint)" }}>
            {name || "(not set)"}
          </Label>
          <Btn small onClick={() => setEditing(true)}>edit</Btn>
        </div>
      )}
    />
  );
};


const PathRow = ({ label, value, hint }) => (
  <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "5px 0", borderBottom: "1px dashed var(--border-dashed-light)" }}>
    <Label style={{ width: 150, flexShrink: 0, fontSize: 14 }}>{label}</Label>
    <Input mono value={value} style={{ flex: 1, padding: "3px 8px", fontSize: 11 }} />
    {hint && <Label hand style={{ fontSize: 12, color: "var(--text-faint)", width: 90, textAlign: "right", flexShrink: 0 }}>{hint}</Label>}
    <Btn small style={{ padding: "2px 8px", fontSize: 13 }}>browse</Btn>
  </div>
);

const AccountCard = ({ name, initial, connected, user }) => (
  <Box style={{ padding: "10px 16px", display: "flex", alignItems: "center", gap: 12 }}>
    <div style={{
      ...sketchyBorder,
      width: 36,
      height: 36,
      background: "var(--shell-bg)",
      boxShadow: "2px 2px 0 var(--shadow)",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
      fontSize: 16,
      fontWeight: 500,
      flexShrink: 0,
    }}>
      {initial}
    </div>
    <Label style={{ fontSize: 14, fontWeight: 500, flexShrink: 0 }}>{name}</Label>
    {connected && (
      <Label hand style={{ color: "var(--text-faint)", fontSize: 13 }}>
        as <strong style={{ color: "var(--text)" }}>{user}</strong>
      </Label>
    )}
    <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 10 }}>
      <Pill tone={connected ? "info" : "neutral"}>
        {connected ? "connected" : "not connected"}
      </Pill>
      <Btn primary={!connected} small>
        {connected ? "disconnect" : "connect"}
      </Btn>
    </div>
  </Box>
);

const ACCOUNTS_DATA = [
  { name: "GitHub", initial: "GH", connected: true, user: "@b2bs" },
  { name: "Nexus Mods", initial: "NX", connected: false },
  { name: "Mega", initial: "M", connected: false },
];

const SettingsScreen = ({ theme = "light", setTheme = () => {} }) => {
  const [tab, setTab] = useState("general");
  const [name, setName] = useState("@b2bs");
  const [language, setLanguage] = useState("English");
  const LANGUAGES = ["English", "German", "French", "Spanish", "Italian", "Polish", "Portuguese", "Czech", "Turkish", "Ukrainian"];
  const tabs = [
    { id: "general", label: "General" },
    { id: "paths", label: "Paths" },
    { id: "tools", label: "Tools" },
    { id: "accounts", label: "Accounts" },
    { id: "advanced", label: "Advanced" },
  ];

  return (
    <div className="sk-page" style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0, padding: "20px 28px 24px" }}>
      <ScreenTitle title="Settings" />
      <div style={{
        display: "flex",
        gap: 6,
        borderBottom: "1.5px solid var(--border-strong)",
        marginBottom: "-1.5px",
        position: "relative",
        zIndex: 1,
        flexShrink: 0,
      }}>
        {tabs.map((tt) => (
          <div
            key={tt.id}
            onClick={() => setTab(tt.id)}
            style={{
              padding: "8px 18px",
              cursor: "pointer",
              fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
              fontSize: 16,
              border: "1.5px solid var(--border-strong)",
              borderBottom: tab === tt.id ? "1.5px solid var(--shell-bg)" : "1.5px solid var(--border-strong)",
              borderRadius: "4px 4px 0 0",
              background: tab === tt.id ? "var(--shell-bg)" : "var(--chrome-bg)",
              marginBottom: "-1.5px",
              fontWeight: tab === tt.id ? 700 : 400,
            }}
          >
            {tt.label}
          </div>
        ))}
      </div>

      <Box style={{ padding: "18px 22px", flex: 1, minHeight: 0, overflow: "auto" }}>
        {tab === "general" && (
          <div>
            <NameRow name={name} setName={setName} />
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "4px 28px" }}>
              <SettingsRow
                label="Theme"
                control={
                  <div style={{ display: "flex", gap: 6 }}>
                    <Btn small primary={theme === "light"} onClick={() => setTheme("light")}>light</Btn>
                    <Btn small primary={theme === "dark"} onClick={() => setTheme("dark")}>dark</Btn>
                  </div>
                }
                hint="light parchment or warm dark"
              />
              <SettingsRow
                label="Language"
                hint="language used across the BIO app"
                control={
                  <select
                    value={language}
                    onChange={(e) => setLanguage(e.target.value)}
                    style={{
                      ...sketchyBorder,
                      padding: "4px 10px",
                      background: "var(--input-bg)",
                      fontFamily: "'Poppins', 'FiraCode Nerd', sans-serif",
                      fontSize: 14,
                      color: "var(--text)",
                      cursor: "pointer",
                      outline: "none",
                    }}
                  >
                    {LANGUAGES.map((l) => <option key={l} value={l}>{l}</option>)}
                  </select>
                }
              />
              <SettingsRow label="Validate all paths on startup" control={<Toggle on={true} />} hint="warns if game folders moved" />
              <SettingsRow label="Diagnostic mode" control={<Toggle on={false} />} hint="extra logging for bug reports" />
            </div>
          </div>
        )}

        {tab === "paths" && (
          <div>
            <Label style={{ color: "var(--text-muted)", marginBottom: 4, fontSize: 13, textTransform: "uppercase", letterSpacing: 0.5 }}>game sources</Label>
            <PathRow label="BGEE source" value="C:\GOG\Baldurs Gate Enhanced Edition" />
            <PathRow label="BG2EE source" value="C:\Steam\steamapps\common\BGII Enhanced Edition" />
            <PathRow label="IWDEE source" value="(not set)" />
            <Label style={{ color: "var(--text-muted)", marginTop: 14, marginBottom: 4, fontSize: 13, textTransform: "uppercase", letterSpacing: 0.5 }}>working folders</Label>
            <PathRow label="Mods archive" value="D:\InfinityOrch\archives" />
            <PathRow label="Mods backup" value="D:\InfinityOrch\backup" />
            <PathRow label="Tools" value="D:\InfinityOrch\tools" />
            <PathRow label="Temp" value="%TEMP%\infinity-orch" />
          </div>
        )}

        {tab === "tools" && (
          <div>
            <Label style={{ color: "var(--text-muted)", marginBottom: 4, fontSize: 13, textTransform: "uppercase", letterSpacing: 0.5 }}>executable paths · auto-detected when possible</Label>
            <PathRow label="weidu.exe" value="D:\InfinityOrch\tools\weidu.exe" hint="v249 ✓" />
            <PathRow label="mod_installer.exe" value="D:\InfinityOrch\tools\mod_installer.exe" />
            <PathRow label="7z executable" value="C:\Program Files\7-Zip\7z.exe" hint="system ✓" />
            <PathRow label="Git executable" value="C:\Program Files\Git\bin\git.exe" hint="2.45.0 ✓" />
          </div>
        )}

        {tab === "accounts" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: 10 }}>
            {ACCOUNTS_DATA.map((a) => (
              <AccountCard key={a.name} {...a} />
            ))}
          </div>
        )}

        {tab === "advanced" && (
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0 28px" }}>
            <div>
              <SectionHead>timing & limits</SectionHead>
              <ValueRow label="Custom scan depth" placeholder="3" />
              <ValueRow label="Mod install timeout" placeholder="7200" hint="sec" />
              <ValueRow label="Auto-answer initial delay" placeholder="4000" hint="ms" />
              <ValueRow label="Auto-answer post-send delay" placeholder="5000" hint="ms" />
              <ValueRow label="Tick (dev)" placeholder="500" hint="ms" />
              <ValueRow label="Prompt context lookback" placeholder="1007" />
            </div>
            <div>
              <SectionHead>install behavior</SectionHead>
              <ToggleRow label="Sound cue when prompt input is required" on={true} />
              <ToggleRow label="Download missing mods and keep archives" on={true} />

              <SectionHead>WeiDU command-line flags</SectionHead>
              <ToggleRow label="-a   Abort on warnings" on={false} />
              <ToggleRow label="-x   Strict matching" on={false} />
              <ToggleRow label="-o   Overwrite mod folder" on={false} />
            </div>
          </div>
        )}
      </Box>
    </div>
  );
};

Object.assign(window, { HomeScreen, ExploreScreen, InstallScreen, CreateScreen, SettingsScreen });
