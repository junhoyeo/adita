// Based on https://github.com/peetzweg/abimate/blob/main/apps/core/src/main.ts
import * as ts from 'typescript';

// https://github.com/Microsoft/TypeScript/wiki/Using-the-Compiler-API#user-content-creating-and-printing-a-typescript-ast
const createLiteralFor = (
  value: any,
):
  | ts.BooleanLiteral
  | ts.NullLiteral
  | ts.StringLiteral
  | ts.ArrayLiteralExpression
  | ts.ObjectLiteralExpression => {
  switch (typeof value) {
    case null:
      return ts.factory.createNull();
    case 'string':
      return ts.factory.createStringLiteral(value);
    case 'boolean':
      return value ? ts.factory.createTrue() : ts.factory.createFalse();
    case 'object':
      if (Array.isArray(value)) {
        return ts.factory.createArrayLiteralExpression(
          value.map((element) => createLiteralFor(element)),
        );
      } else {
        return createObjectFromObject(value);
      }
    default:
      return ts.factory.createStringLiteral('not yet implemented');
  }
};

const createObjectFromObject = (fragment: object) => {
  const properties = Object.entries(fragment).map(([key, value]) => {
    return ts.factory.createPropertyAssignment(key, createLiteralFor(value));
  });
  return ts.factory.createObjectLiteralExpression(properties);
};

export interface Fragment {
  name?: string;
  type: 'receive' | 'constructor' | 'fallback';
  inputs: Array<{
    name?: string;
    type: string;
  }>;
}

const createFragmentDeclaration = (
  fragment: Fragment,
  options?: { explicitIdentifier?: boolean },
): [ts.Identifier, ts.ParameterDeclaration] => {
  let identifierString = fragment['name'];

  if (!identifierString) {
    throw new Error(
      `Unable to create Identifier for Fragment: ${JSON.stringify(fragment)}`,
    );
  }

  if (identifierString && options?.explicitIdentifier) {
    identifierString = `${fragment['name']}_${fragment.inputs
      .map((input) => input.type.replace('[]', 'Array'))
      .join('_')}`;
  }

  const identifier = ts.factory.createIdentifier(identifierString);

  const expression = ts.factory.createParameterDeclaration(
    [
      ts.factory.createToken(ts.SyntaxKind.ExportKeyword),
      ts.factory.createToken(ts.SyntaxKind.ConstKeyword),
    ],
    undefined,
    identifier,
    undefined,
    undefined,
    ts.factory.createAsExpression(
      createObjectFromObject(fragment),
      ts.factory.createKeywordTypeNode(ts.SyntaxKind.ConstKeyword as any),
    ),
  );

  return [identifier, expression];
};

export const createContractFileForAbi = (
  abis: Array<Fragment>,
): Array<ts.Node> => {
  const filteredAbi: any[] = abis.filter((fragment) => !!fragment.name);
  const nameCounts = filteredAbi
    .map((fragment) => fragment.name)
    .reduce((acc, cur) => {
      if (acc[cur]) {
        acc[cur] += 1;
      } else {
        acc[cur] = 1;
      }
      return acc;
    }, []);

  const fragmentDeclarationIdentifiers: Array<ts.Identifier> = [];
  const fragmentDeclarations: Array<ts.ParameterDeclaration> = filteredAbi.map(
    (fragment) => {
      const [identifier, declaration] = createFragmentDeclaration(fragment, {
        explicitIdentifier: nameCounts[fragment.name] > 1,
      });
      fragmentDeclarationIdentifiers.push(identifier);
      return declaration;
    },
  );

  if (fragmentDeclarationIdentifiers.length === 0) return [];

  const exportDefault = ts.factory.createExportAssignment(
    [ts.factory.createToken(ts.SyntaxKind.DefaultKeyword)],
    false,
    ts.factory.createAsExpression(
      ts.factory.createArrayLiteralExpression(fragmentDeclarationIdentifiers),
      ts.factory.createKeywordTypeNode(ts.SyntaxKind.ConstKeyword as any),
    ),
  );

  return [...fragmentDeclarations, exportDefault];
};
