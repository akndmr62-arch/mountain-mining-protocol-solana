export default function HowItWorksPage() {
  return (
    <div className="card">
      <h1>How It Works</h1>
      <ol className="muted">
        <li>Protocol mints a Mining Pass NFT in one of the fixed issuance phases.</li>
        <li>User starts mining, moving the NFT into PDA custody.</li>
        <li>Rewards accrue from wall-clock time using integer-only reward math.</li>
        <li>User claims at any time or stops mining to auto-claim and receive the NFT back.</li>
      </ol>
    </div>
  );
}
