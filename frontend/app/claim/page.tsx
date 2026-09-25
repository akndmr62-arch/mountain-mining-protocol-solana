import { ActionPanel } from "@/components/action-panel";
import { getIdlStatus } from "@/lib/idl";

export default function ClaimPage() {
  const status = getIdlStatus();
  return <ActionPanel title="Claim Rewards" method="claim" wired={status.wired} reason={status.reason} />;
}
