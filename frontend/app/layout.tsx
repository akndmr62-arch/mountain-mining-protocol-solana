import type { Metadata } from "next";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { AppWalletProvider } from "@/components/wallet-provider";
import { Nav } from "@/components/nav";
import "./globals.css";

export const metadata: Metadata = {
  title: "Mountain Mining Protocol",
  description: "Devnet-ready Solana/Anchor NFT mining protocol frontend skeleton.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <AppWalletProvider>
          <main>
            <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem", alignItems: "center", marginBottom: "1rem" }}>
              <div>
                <div className="pill">Devnet-ready only · not mainnet deployed</div>
                <h1>Mountain Mining Protocol</h1>
              </div>
              <WalletMultiButton />
            </div>
            <Nav />
            {children}
          </main>
        </AppWalletProvider>
      </body>
    </html>
  );
}
