/**
 * Rust Emitter for JiraDb TypeSpec
 *
 * Generates Axum-based Rust code from TypeSpec definitions.
 */

import { EmitContext } from "@typespec/compiler";
import { parseProgram } from "@jira-db/emitter-common";
import { generateRust } from "./generator.js";
import * as fs from "fs";

export async function $onEmit(context: EmitContext) {
  const { program, emitterOutputDir } = context;

  // Parse TypeSpec into IR
  const schema = parseProgram(program);

  // Generate Rust code from IR
  const files = generateRust(schema);

  // Ensure output directory exists
  if (!fs.existsSync(emitterOutputDir)) {
    fs.mkdirSync(emitterOutputDir, { recursive: true });
  }

  // Write output files
  for (const [filename, content] of Object.entries(files)) {
    await program.host.writeFile(`${emitterOutputDir}/${filename}`, content);
  }
}
