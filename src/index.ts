import { glob } from 'fast-glob';
import { readFile } from 'fs/promises';
import { join as joinPath } from 'path';

process.on('uncaughtException', function (err) {
  console.error(err);
  process.exit(1);
});

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
        console.log(abis);
      }
    }),
  );
};

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
