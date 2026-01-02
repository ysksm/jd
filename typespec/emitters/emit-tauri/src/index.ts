/**
 * Tauri Emitter for JiraDb TypeSpec
 *
 * Generates Tauri command code from TypeSpec definitions.
 */

import { EmitContext } from "@typespec/compiler";
import { parseProgram } from "@jira-db/emitter-common";
import { generateTauri } from "./generator.js";
import * as fs from "fs";

export async function $onEmit(context: EmitContext) {
  const { program, emitterOutputDir } = context;

  console.log(`[emit-tauri] Output directory: ${emitterOutputDir}`);

  // Parse TypeSpec into IR
  const schema = parseProgram(program);

  console.log(`[emit-tauri] Parsed ${schema.models.length} models, ${schema.namespaces.length} namespaces`);

  // Generate Tauri code from IR
  const files = generateTauri(schema);

  console.log(`[emit-tauri] Generated ${Object.keys(files).length} files: ${Object.keys(files).join(', ')}`);

  // Ensure output directory exists
  if (!fs.existsSync(emitterOutputDir)) {
    fs.mkdirSync(emitterOutputDir, { recursive: true });
  }

  // Write output files
  for (const [filename, content] of Object.entries(files)) {
    const fullPath = `${emitterOutputDir}/${filename}`;
    console.log(`[emit-tauri] Writing ${fullPath}`);
    await program.host.writeFile(fullPath, content);
  }

  console.log(`[emit-tauri] Done`);
}
