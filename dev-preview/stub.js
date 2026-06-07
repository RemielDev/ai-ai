// Stub for window.__TAURI__ so the UI renders without a real Tauri backend.
// Used only for visual mockup generation via Playwright.

window.__TAURI__ = {
  core: {
    async invoke(cmd, args) {
      console.log("[stub] invoke", cmd, args);
      switch (cmd) {
        case "get_api_key_status":
          return window.__STUB_STATE__?.keyPresent ?? true;
        case "license_status":
          return { valid: true, tier: "dev-bypass", message: "License gate disabled in this build." };
        case "load_settings":
          return {
            provider: "gemini",
            model: "gemini-2.0-flash",
            summon_hotkey: "Ctrl+Shift+Space",
            action_hotkey: "Ctrl+Shift+Enter",
            suggestion_count: 4,
            style_preset: "default",
            auto_send_after_paste: false,
            telemetry_opt_in: false,
            first_run: false,
            first_launch_at: new Date().toISOString(),
          };
        case "get_autostart":   return false;
        case "trial_status":    return { in_trial: true, days_left: 5 };
        case "get_app_info":    return { version: "0.1.0", suggestions_today: 23 };
        case "validate_api_key": return null;
        case "check_for_updates": return "You're on the latest version (0.1.0).";
        default: return null;
      }
    },
  },
  event: {
    async listen(name, handler) {
      window.__STUB_LISTENERS__ ??= {};
      window.__STUB_LISTENERS__[name] = handler;
      return () => {};
    },
  },
  window: {
    getCurrentWindow: () => ({
      setFocus: async () => {},
      close:    async () => {},
    }),
  },
  opener: { openUrl: async (url) => console.log("[stub] openUrl", url) },
};

window.__STUB_EMIT__ = function (name, payload) {
  const fn = (window.__STUB_LISTENERS__ || {})[name];
  if (fn) fn({ payload });
};
