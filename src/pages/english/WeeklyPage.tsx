export function WeeklyPage() {
  return (
    <>
      <h2 className="page-title">周短文</h2>
      <p className="page-lead">阅读区使用更窄栏宽（约 38rem）与更大段间距。</p>
      <div className="card" style={{ maxWidth: "38rem" }}>
        <p style={{ fontFamily: "var(--font-serif)", fontSize: "1.125rem" }}>
          本周短文占位。正式内容从本地存储或静态资源加载。
        </p>
      </div>
    </>
  );
}
