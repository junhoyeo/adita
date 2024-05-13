# adita

WIP, Ethereum SDK for the next generation

- [peetzweg/abimate](https://github.com/peetzweg/abimate)

## cli

```bash
adita --source ./.../artifacts --out-dir ./abis
```

## tools

```tsx
import { TXButton, finalized } from '?';

const Home = () => {
  const handler = useCallback(async () => {
    try {
      const promise = writeContractAsync({
        ...contract,
        functionName: 'approve',
        args: [Contracts.Core, parseEther('1.0')],
      });
      const hash = await finalized(toast(promise));
      console.log(hash);
    } finally {
      await refresh();
    }
  }, [refresh]);

  return (
    <TXButton
      handler={handler}
      title={{
        default: 'Approve Tokens',
        loading: 'Approving',
      }}
    />
  );
};
```
