import { ActionPanel } from "@/components/action-panel";
import { getIdlStatus } from "@/lib/idl";

export default function MinePage() {
  const status = getIdlStatus();
  return <ActionPanel title="Start Mining" method="startMining" wired={status.wired} reason={status.reason} />;
}
