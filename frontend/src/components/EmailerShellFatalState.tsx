import { ShellFatalState } from "@pushkind/frontend-shell/ShellFatalState";

type EmailerShellFatalStateProps = {
  message: string;
};

export function EmailerShellFatalState({
  message,
}: EmailerShellFatalStateProps) {
  return (
    <ShellFatalState
      message={message}
      serviceLabel="pushkind-emailer"
      title="Не удалось загрузить оболочку"
      shellClassName="foundation-page"
      cardClassName="foundation-card"
      eyebrowClassName="foundation-eyebrow"
      titleClassName="foundation-title"
      messageClassName="foundation-copy"
    />
  );
}
