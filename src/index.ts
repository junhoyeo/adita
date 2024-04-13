import { glob } from 'fast-glob';
import * as fs from 'fs';
import * as path from 'path';
import * as ts from 'typescript';
import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';

import { createContractFileForAbi } from './codegen';

process.on('uncaughtException', function (err) {
  console.error(err);
  process.exit(1);
});

const printer = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed });

const main = async () => {
  const argv = await yargs(hideBin(process.argv))
    .option('source', { type: 'string' })
    .demandOption('source')
    .option('out-dir', { type: 'string' })
    .default('out-dir', './abis')
    .parse();

  const source = path.join(argv.source, '**/*.json');
  const entries = await glob([source], {
    dot: true,
    ignore: ['**/*.dbg.json'],
  });

  if (!fs.existsSync(argv.outDir)) {
    await fs.promises.mkdir(argv.outDir, { recursive: true });
  }

  await Promise.all(
    entries.map(async (entry) => {
      const data = JSON.parse(await fs.promises.readFile(entry, 'utf8'));
      if (Object.hasOwn(data, 'abi')) {
        const abis = data.abi;
        console.log(entry);

        const lineNodes = createContractFileForAbi(abis);
        if (Array.isArray(lineNodes) && lineNodes.length === 0) {
          return [];
        }

        const fileName = path.basename(entry, path.extname(entry));
        const outputFile = path.join(argv.outDir, `${fileName}.ts`);
        const fileObj = ts.createSourceFile(
          outputFile,
          '',
          ts.ScriptTarget.ESNext,
          false,
          ts.ScriptKind.TS,
        );
        const lineStrings = lineNodes.map((node) =>
          printer.printNode(ts.EmitHint.Unspecified, node, fileObj),
        );
        await fs.promises.writeFile(outputFile, lineStrings.join('\n'));
      }
    }),
  );
};

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
