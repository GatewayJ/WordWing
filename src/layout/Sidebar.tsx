import { NavLink } from "react-router-dom";

type SidebarProps = {
  englishActive: boolean;
  todoActive: boolean;
  className?: string;
  showFooterLink?: boolean;
};

export function Sidebar({ englishActive, todoActive, className, showFooterLink }: SidebarProps) {
  return (
    <aside className={className ?? "sidebar"}>
      <nav aria-label="主导航">
        <div className="sidebar__group">
          <div className={`sidebar__label${englishActive ? " sidebar__label--current" : ""}`}>
            英语
          </div>
          <NavLink
            to="/english/vocabulary"
            className={({ isActive }) =>
              `sidebar__link${isActive ? " sidebar__link--active" : ""}`
            }
          >
            生词
          </NavLink>
          <NavLink
            to="/english/review"
            className={({ isActive }) =>
              `sidebar__link${isActive ? " sidebar__link--active" : ""}`
            }
          >
            复习
          </NavLink>
          <NavLink
            to="/english/weekly"
            className={({ isActive }) =>
              `sidebar__link${isActive ? " sidebar__link--active" : ""}`
            }
          >
            周短文
          </NavLink>
        </div>

        <div className="sidebar__group">
          <div className={`sidebar__label${todoActive ? " sidebar__label--current" : ""}`}>Todo</div>
          <NavLink
            to="/todo/items"
            className={({ isActive }) =>
              `sidebar__link${isActive ? " sidebar__link--active" : ""}`
            }
          >
            条目
          </NavLink>
          <NavLink
            to="/todo/schedules"
            className={({ isActive }) =>
              `sidebar__link${isActive ? " sidebar__link--active" : ""}`
            }
          >
            定时
          </NavLink>
        </div>
      </nav>

      {showFooterLink && (
        <div className="sidebar__footer">
          <NavLink to="/settings" className="sidebar__footer-link">
            设置
          </NavLink>
        </div>
      )}
    </aside>
  );
}
