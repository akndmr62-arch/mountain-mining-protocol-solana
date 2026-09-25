import fs from "node:fs";
import { NextResponse } from "next/server";
import { getIdlStatus } from "@/lib/idl";

export async function GET() {
  const status = getIdlStatus();
  if (!status.wired) {
    return NextResponse.json(status, { status: 404 });
  }
  return NextResponse.json(JSON.parse(fs.readFileSync(status.path, "utf8")));
}
