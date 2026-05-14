/* global React, ReactDOM, HomeScreen, ExploreScreen, InstallScreen, CreateScreen, SettingsScreen, useTweaks, TweaksPanel, TweakSection, TweakRadio, TweakToggle, TweakColor, TweakSelect */
const { useState } = React;

const NAV_V2 = [
  { id: "home", label: "Home", icon: "⌂" },
  { id: "explore", label: "Explore", icon: "✦" },
  { id: "install", label: "Install", icon: "↓" },
  { id: "create", label: "Create", icon: "✎" },
  { id: "settings", label: "Settings", icon: "⚙" },
];
const NAV_V1 = NAV_V2.filter((n) => n.id !== "explore");

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "version": "v1",
  "navStyle": "labels",
  "density": "comfy",
  "accent": "#14B8A6",
  "showAnnotations": true
}/*EDITMODE-END*/;

const ACCENTS = ["#14B8A6", "#4ADE80", "#60a5fa", "#fbbf24"];

function App() {
  const [active, setActive] = useState("home");
  const [theme, setTheme] = useState("dark");
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
  // Resumed in-progress build: when set, Create renders the workspace pre-populated.
  const [resumedBuild, setResumedBuild] = useState(null);
  // Reinstall: when set, Install screen opens pre-populated at the preview stage with
  // the modlist's stored share code, destination, and overwrite-install mode forced.
  const [reinstallTarget, setReinstallTarget] = useState(null);
  const navigate = (dest) => {
    setActive(dest);
    if (dest !== "create") setResumedBuild(null);
    if (dest !== "install") setReinstallTarget(null);
  };
  const resumeBuild = (build) => {
    setResumedBuild(build);
    setActive("create");
  };
  const reinstallBuild = (build) => {
    setReinstallTarget(build);
    setActive("install");
  };

  // apply accent
  const accentDeep = darken(t.accent);
  const cssVars = {
    "--accent": t.accent,
    "--accent-deep": accentDeep,
  };

  const navWidth = t.navStyle === "icons" ? 64 : 200;
  const isV1 = t.version === "v1";
  const NAV = isV1 ? NAV_V1 : NAV_V2;

  React.useEffect(() => {
    if (isV1 && active === "explore") setActive("home");
  }, [isV1, active]);

  React.useEffect(() => {
    document.body.dataset.theme = theme;
    return () => { delete document.body.dataset.theme; };
  }, [theme]);

  const renderScreen = () => {
    switch (active) {
      case "home": return <HomeScreen v1={isV1} navigate={navigate} resumeBuild={resumeBuild} reinstallBuild={reinstallBuild} />;
      case "explore": return isV1 ? <HomeScreen v1 navigate={navigate} resumeBuild={resumeBuild} reinstallBuild={reinstallBuild} /> : <ExploreScreen />;
      case "install": return <InstallScreen reinstallTarget={reinstallTarget} />;
      case "create": return <CreateScreen resumedBuild={resumedBuild} resumeBuild={resumeBuild} />;
      case "settings": return <SettingsScreen theme={theme} setTheme={setTheme} />;
      default: return null;
    }
  };

  return (
    <div className="sk-shell" style={cssVars}>
      {/* window chrome */}
      <div className="sk-titlebar">
        <div className="sk-traffic">
          <span className="dot" />
          <span className="dot" />
          <span className="dot" />
        </div>
        <div className="sk-title">Infinity Orchestrator <span style={{fontFamily:"'JetBrains Mono',monospace",fontSize:11,color:"#888",marginLeft:6}}>{isV1 ? "· v1" : "· v2"}</span></div>
        <div className="sk-window-ctrls">
          <span>—</span>
          <span>▢</span>
          <span>×</span>
        </div>
      </div>

      {/* main body */}
      <div className="sk-body" data-density={t.density}>
        {/* left nav */}
        <aside className="sk-nav" style={{ width: navWidth }}>
          <div className="sk-brand">
            <div className="sk-brand-mark">∞</div>
            {t.navStyle === "labels" && (
              <div className="sk-brand-text">
                <div className="sk-brand-name">Infinity</div>
                <div className="sk-brand-sub">Orchestrator</div>
              </div>
            )}
          </div>

          <nav className="sk-nav-items">
            {NAV.map((n) => (
              <button
                key={n.id}
                className={`sk-nav-item ${active === n.id ? "active" : ""}`}
                onClick={() => navigate(n.id)}
                title={n.label}
              >
                <span className="sk-nav-icon">{n.icon}</span>
                {t.navStyle === "labels" && <span className="sk-nav-label">{n.label}</span>}
              </button>
            ))}
          </nav>

          <div className="sk-nav-foot">
            {t.navStyle === "labels" ? (
              <>
                <div className="sk-status-dot" />
                <span>weidu v249 · all paths ok</span>
              </>
            ) : (
              <div className="sk-status-dot" title="all paths ok" />
            )}
          </div>

          {t.showAnnotations && t.navStyle === "labels" && (
            <div className="sk-annotation sk-anno-nav">
              <span className="sk-anno-arrow">↗</span>
              persistent left rail · 5 top-level destinations
            </div>
          )}
        </aside>

        {/* right panel */}
        <main className="sk-main">
          {renderScreen()}
        </main>
      </div>

      {/* status / footer bar */}
      <div className="sk-statusbar">
        <span>● connected</span>
        <span>·</span>
        <span>3 modlists</span>
        <span>·</span>
        <span>0 jobs running</span>
        <span style={{ marginLeft: "auto" }}>v0.1.0 · wireframe</span>
      </div>

      <TweaksPanel title="Tweaks">
        <TweakSection title="Version">
          <TweakRadio label="Release" tweakKey="version" value={t.version} onChange={setTweak}
            options={[{ label: "v1 (offline)", value: "v1" }, { label: "v2 (community)", value: "v2" }]} />
        </TweakSection>
        <TweakSection title="Layout">
          <TweakRadio label="Nav style" tweakKey="navStyle" value={t.navStyle} onChange={setTweak}
            options={[{ label: "Icons + labels", value: "labels" }, { label: "Icons only", value: "icons" }]} />
          <TweakRadio label="Density" tweakKey="density" value={t.density} onChange={setTweak}
            options={[{ label: "Comfy", value: "comfy" }, { label: "Compact", value: "compact" }]} />
        </TweakSection>
        <TweakSection title="Style">
          <TweakColor label="Accent" tweakKey="accent" value={t.accent} onChange={setTweak} options={ACCENTS} />
          <TweakToggle label="Show annotations" tweakKey="showAnnotations" value={t.showAnnotations} onChange={setTweak} />
        </TweakSection>
      </TweaksPanel>
    </div>
  );
}

function darken(hex) {
  // simple darken for accent-deep
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  const f = 0.6;
  const dr = Math.round(r * f), dg = Math.round(g * f), db = Math.round(b * f);
  return `rgb(${dr}, ${dg}, ${db})`;
}

ReactDOM.createRoot(document.getElementById("root")).render(<App />);
