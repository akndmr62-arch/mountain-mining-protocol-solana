import fs from "node:fs";
import path from "node:path";

const idlPath = path.resolve(process.cwd(), "../target/idl/mountain_mining.json");

export function getIdlStatus() {
  if (!fs.existsSync(idlPath)) {
    return {
      wired: false,
      reason: `IDL not found at ${idlPath}. Run anchor build first.`,
      path: idlPath,
    };
  }

  try {
    const parsed = JSON.parse(fs.readFileSync(idlPath, "utf8"));
    return {
      wired: true,
      reason: "IDL detected and action routes are wired to the Anchor program.",
      path: idlPath,
      programId: parsed.address ?? parsed.metadata?.address ?? null,
    };
  } catch (error) {
    return {
      wired: false,
      reason: `Failed to parse IDL at ${idlPath}: ${String(error)}`,
      path: idlPath,
    };
  }
}
