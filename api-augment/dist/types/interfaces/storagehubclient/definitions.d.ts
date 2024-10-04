declare const _default: {
  types: import("@polkadot/types/types").RegistryTypes | undefined;
  runtime: import("@polkadot/types/types").DefinitionsCall | undefined;
  rpc:
    | Record<
        string,
        | import("@polkadot/types/types").DefinitionRpc
        | import("@polkadot/types/types").DefinitionRpcSub
      >
    | undefined;
};
export default _default;
