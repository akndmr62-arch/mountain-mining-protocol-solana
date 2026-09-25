import Link from "next/link";

const routes = [
  ["/", "Overview"],
  ["/mine", "Mine"],
  ["/claim", "Claim"],
  ["/stop", "Stop"],
  ["/statistics", "Statistics"],
  ["/faq", "FAQ"],
  ["/project-info", "Project Info"],
  ["/how-it-works", "How It Works"],
] as const;

export function Nav() {
  return (
    <nav>
      {routes.map(([href, label]) => (
        <Link key={href} href={href}>{label}</Link>
      ))}
    </nav>
  );
}
