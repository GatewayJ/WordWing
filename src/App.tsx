import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./layout/AppShell";
import { CollectionPage } from "./pages/english/CollectionPage";
import { VocabularyPage } from "./pages/english/VocabularyPage";
import { ReviewPage } from "./pages/english/ReviewPage";
import { WeeklyPage } from "./pages/english/WeeklyPage";
import { ItemsPage } from "./pages/todo/ItemsPage";
import { SchedulesPage } from "./pages/todo/SchedulesPage";
import { SettingsPage } from "./pages/SettingsPage";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<AppShell />}>
          <Route index element={<Navigate to="/english/vocabulary" replace />} />
          <Route path="english/vocabulary" element={<VocabularyPage />} />
          <Route path="english/collection" element={<CollectionPage />} />
          <Route path="english/review" element={<ReviewPage />} />
          <Route path="english/weekly" element={<WeeklyPage />} />
          <Route path="todo/items" element={<ItemsPage />} />
          <Route path="todo/schedules" element={<SchedulesPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/english/vocabulary" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
