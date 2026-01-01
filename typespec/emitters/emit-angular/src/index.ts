/**
 * Angular Emitter for JiraDb TypeSpec
 *
 * Generates Angular TypeScript code from TypeSpec definitions.
 */

import { EmitContext } from "@typespec/compiler";
import { parseProgram } from "@jira-db/emitter-common";
import { generateAngular } from "./generator.js";

export async function $onEmit(context: EmitContext) {
  const { program, emitterOutputDir } = context;

  // Parse TypeSpec into IR
  const schema = parseProgram(program);

  // Generate Angular code from IR
  const files = generateAngular(schema);

  // Write output files
  for (const [filename, content] of Object.entries(files)) {
    await program.host.writeFile(`${emitterOutputDir}/${filename}`, content);
  }
}
