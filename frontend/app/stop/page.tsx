import { ActionPanel } from "@/components/action-panel";
import { getIdlStatus } from "@/lib/idl";

export default function StopPage() {
  const status = getIdlStatus();
  return <ActionPanel title="Stop Mining" method="stopMining" wired={status.wired} reason={status.reason} />;
}
