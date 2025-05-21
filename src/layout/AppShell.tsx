import { useCallback, useEffect, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { Menu, Moon, Settings, Sun } from "lucide-react";
import { Sidebar } from "./Sidebar";

function pathMatchesModule(pathname: string, prefix: string) {
  return pathname === prefix || pathname.startsWith(`${prefix}/`);
}

export function AppShell() {
  const location = useLocation();
  const navigate = useNavigate();
  const [theme, setTheme] = useState<"light" | "dark">(() =>
    document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light",
  );
  const [mobileOpen, setMobileOpen] = useState(false);
  const [narrow, setNarrow] = useState(
    () => typeof window !== "undefined" && window.matchMedia("(max-width: 719px)").matches,
  );

  useEffect(() => {
    const mq = window.matchMedia("(max-width: 719px)");
    const onChange = () => setNarrow(mq.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme === "dark" ? "dark" : "light");
    try {
      localStorage.setItem("wordwing-theme", theme);
    } catch {
      /* ignore */
    }
  }, [theme]);

  useEffect(() => {
    try {
      const stored = localStorage.getItem("wordwing-theme");
      if (stored === "dark" || stored === "light") setTheme(stored);
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    setMobileOpen(false);
  }, [location.pathname]);

  const toggleTheme = useCallback(() => {
    setTheme((t) => (t === "dark" ? "light" : "dark"));
  }, []);

  const englishActive = pathMatchesModule(location.pathname, "/english");
  const todoActive = pathMatchesModule(location.pathname, "/todo");

  return (
    <div className="app-root">
      <header className="top-bar">
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          {narrow && (
            <button
              type="button"
              className="icon-btn"
              aria-label="打开导航菜单"
              onClick={() => setMobileOpen(true)}
            >
              <Menu size={20} strokeWidth={2} />
            </button>
          )}
          <h1 className="top-bar__brand">WordWing</h1>
        </div>
        <div className="top-bar__actions">
          <button
            type="button"
            className="icon-btn"
            aria-label={theme === "dark" ? "切换到浅色" : "切换到深色"}
            onClick={toggleTheme}
          >
            {theme === "dark" ? <Sun size={20} /> : <Moon size={20} />}
          </button>
          <button
            type="button"
            className="icon-btn"
            aria-label="设置"
            data-active={location.pathname === "/settings" ? "true" : undefined}
            onClick={() => navigate("/settings")}
          >
            <Settings size={20} />
          </button>
        </div>
      </header>

      <div className="app-body">
        {narrow && mobileOpen && (
          <button
            type="button"
            className="sidebar-backdrop"
            aria-label="关闭菜单"
            onClick={() => setMobileOpen(false)}
          />
        )}
        <Sidebar
          englishActive={englishActive}
          todoActive={todoActive}
          className={narrow ? (mobileOpen ? "sidebar sidebar--open" : "sidebar") : "sidebar"}
          showFooterLink={narrow}
        />
        <main className="main-area">
          <div className="main-area__inner">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
