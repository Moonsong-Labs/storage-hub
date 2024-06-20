export const nodeInfo = {
  user: {
    containerName: "docker-sh-user-1",
    port: 9977,
    p2pPort: 30444,
    AddressId: "5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o",
    expectedPeerId: "12D3KooWMvbhtYjbhgjoDzbnf71SFznJAKBBkSGYEUtnpES1y9tM",
  },
  bsp: {
    containerName: "docker-sh-bsp-1",
    port: 9966,
    p2pPort: 30350,
    AddressId: "5FHSHEFWHVGDnyiw66DoRUpLyh5RouWkXo9GT1Sjk8qw7MAg",
    expectedPeerId: "12D3KooWNEZ8PGNydcdXTYy1SPHvkP9mbxdtTqGGFVrhorDzeTfU",
  },
  collator: {
    containerName: "docker-sh-collator-1",
    port: 9955,
    p2pPort: 30333,
    AddressId: "5C8NC6YuAivp3knYC58Taycx2scQoDcDd3MCEEgyw36Gh1R4",
  },
} as const;