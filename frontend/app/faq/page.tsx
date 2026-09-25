export default function FaqPage() {
  return (
    <div className="card">
      <h1>FAQ</h1>
      <p className="muted"><strong>Is this mainnet live?</strong> No. This repository is devnet-ready only.</p>
      <p className="muted"><strong>Can admins mint MMP?</strong> No. Only the mint-authority PDA can mint MMP, and only claim/stop paths invoke it.</p>
      <p className="muted"><strong>How are classes assigned?</strong> Weighted by remaining class capacity using SlotHashes-based entropy, which must be replaced with VRF before any mainnet use.</p>
    </div>
  );
}
