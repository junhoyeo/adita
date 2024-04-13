import { glob } from 'fast-glob';
import { writeFileSync } from 'fs';
import { readFile } from 'fs/promises';
import { basename as basenamePath, extname, join as joinPath } from 'path';
import * as ts from 'typescript';

import { createContractFileForAbi } from './codegen';

process.on('uncaughtException', function (err) {
  console.error(err);
  process.exit(1);
});

const printer = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed });

const main = async () => {
  console.log(process.argv);
  const input = process.argv[2];
  const source = joinPath(input, '**/*.json');

  const entries = await glob([source], {
    dot: true,
    ignore: ['**/*.dbg.json'],
  });

  await Promise.all(
    entries.map(async (entry) => {
      const data = JSON.parse(await readFile(entry, 'utf8'));
      if (Object.hasOwn(data, 'abi')) {
        const abis = data.abi;
        console.log(entry);

        const lineNodes = createContractFileForAbi(abis);
        if (Array.isArray(lineNodes) && lineNodes.length === 0) {
          return [];
        }

        const fileName = basenamePath(entry, extname(entry));
        const outputFile = `./out/${fileName}.ts`;
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
        writeFileSync(outputFile, lineStrings.join('\n'));
      }
    }),
  );
};

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
