import { glob } from 'fast-glob';
import * as fs from 'fs';
import * as path from 'path';
import ts from 'typescript';
import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';

import { Fragment, createContractFileForAbi } from './codegen';

process.on('uncaughtException', function (err) {
  console.error(err);
  process.exit(1);
});

const printer = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed });

const getAbiKey = (abi: Fragment): string => {
  const name = abi.name || '';
  const type = abi.type || '';

  const inputTypes = abi.inputs
    ? abi.inputs
        .map((input) => input.type)
        .sort()
        .join(',')
    : '';

  const outputTypes = abi.outputs
    ? abi.outputs
        .map((output) => output.type)
        .sort()
        .join(',')
    : '';

  return `${name}:${type}:${inputTypes}:${outputTypes}`;
};

const removeDuplicateAbis = (abis: Fragment[]): Fragment[] => {
  const uniqueAbis: Fragment[] = [];
  const seen = new Set<string>();

  for (const abi of abis) {
    const key = getAbiKey(abi);

    if (!seen.has(key)) {
      seen.add(key);
      uniqueAbis.push(abi);
    }
  }

  return uniqueAbis;
};

const main = async (): Promise<void> => {
  const argv = await yargs(hideBin(process.argv))
    .option('source', { type: 'string' })
    .demandOption('source')
    .option('out-dir', { type: 'string', alias: 'outDir' })
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

  const abisByOutputFile = new Map<string, Fragment[]>();

  for (const entry of entries) {
    try {
      const fileContent = await fs.promises.readFile(entry, 'utf8');
      const data = JSON.parse(fileContent);

      if (!Object.hasOwn(data, 'abi')) {
        continue;
      }

      const abis: Fragment[] = data.abi;
      const fileName = path.basename(entry, path.extname(entry));
      const outputFile = path.join(argv.outDir as string, `${fileName}.ts`);

      if (!abisByOutputFile.has(outputFile)) {
        abisByOutputFile.set(outputFile, []);
      }

      const existingAbis = abisByOutputFile.get(outputFile) || [];
      abisByOutputFile.set(outputFile, [...existingAbis, ...abis]);
    } catch (error) {
      console.error(`Error processing ${entry}:`, error);
    }
  }

  for (const [outputFile, abis] of abisByOutputFile.entries()) {
    try {
      const uniqueAbis = removeDuplicateAbis(abis);
      const lineNodes = createContractFileForAbi(uniqueAbis);
      if (Array.isArray(lineNodes) && lineNodes.length === 0) {
        continue;
      }

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
    } catch (error) {
      console.error(`Error generating ${outputFile}:`, error);
    }
  }
};

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
