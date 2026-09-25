import Link from "next/link";
import { getIdlStatus } from "@/lib/idl";

export default function HomePage() {
  const status = getIdlStatus();
  return (
    <div className="grid">
      <section className="card">
        <h2>Overview</h2>
        <p className="muted">
          MMP is an NFT-based passive mining protocol on Solana. Mining Pass NFTs are locked into
          protocol custody while mining, rewards accrue with integer-only accounting, and the global
          supply cap is enforced through non-reverting claim-time clamping.
        </p>
        <div className="status">
          <strong>{status.wired ? "IDL detected" : "IDL missing"}</strong>: {status.reason}
        </div>
      </section>
      <section className="grid grid-2">
        {["mine", "claim", "stop", "statistics"].map((slug) => (
          <Link className="card" key={slug} href={`/${slug}`}>
            <h3 style={{ textTransform: "capitalize" }}>{slug}</h3>
            <p className="muted">Open the {slug} flow.</p>
          </Link>
        ))}
      </section>
    </div>
  );
}
