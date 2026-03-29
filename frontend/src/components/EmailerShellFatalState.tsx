type EmailerShellFatalStateProps = {
  message: string;
};

export function EmailerShellFatalState({
  message,
}: EmailerShellFatalStateProps) {
  return (
    <main className="foundation-page">
      <div className="foundation-card">
        <p className="foundation-eyebrow">pushkind-emailer</p>
        <h1 className="foundation-title">Не удалось загрузить оболочку</h1>
        <p className="foundation-copy">{message}</p>
      </div>
    </main>
  );
}
