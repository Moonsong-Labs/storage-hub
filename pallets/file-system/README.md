# Pallet File System Pallet

## Design

### File Deletion

As a reminder, files stored by providers, be it BSPs or MSPs, can only be removed from a provider's _forest_ once they provide a proof of inclusion. The runtime is responsible for verifying the proof and removing the leaf (file) from the _forest_.

User's who wish to delete a file will initially call the `delete_file` extrinsic. This extrinsic optionally accepts a proof of inclusion of the file in a bucket's _forest_. MSPs will expose an RPC endpoint to allow users to request a proof of inclusion of a file in their _forest_.

If the proof is provided, the runtime will verify the proof and queue a priority challenge in the proofs-dealer pallet paired with `TrieRemoveMutation`, forcing all providers to provide a proof of (non-)inclusion of the challenged file. The runtime will automatically remove the file from the _forest_ for all providers who submitted a proof of inclusion.

If the proof is not provided, a pending file deletion request is created with a defined constant expiration time. The MSP who is supposed to be storing the file, will have until the expiration time to provide a proof of (non-)inclusion of the file in their _forest_. If it is a proof of non-inclusion, this means the file does not exist, and the pending file deletion request will be removed. This prevents all the providers from responding to a priority challenge for a file that does not exist. If it is a proof of inclusion, a priority challenge will be queued in the proofs-dealer pallet, following the same process described above.

Once a pending file deletion request reaches its expiration time and it has not been responded to, a priority challenge will be queued, following the same process described above.

### Volunteering: Succeeding Threshold Checks

BSPs are required to volunteer to store data for storage requests. To ensure data is probabilistically stored evenly across all BSPs, a threshold must be succeeded by each volunteering BSP.
The threshold to succeed is different for every BSP since it is based on their reputation weight. Gaining more reputation will increase the probability of succeeding the threshold check.

The computed threshold result of a BSP is formalized as $H(C(P, F))$ where $H$ is the hashing function, $C$ is the concatenation function, $P$ is the provider id and $F$ is the file key.
The $n$ least significant bits of the hash result are converted into the configured runtime's threshold type, where $n$ is the number of bits required to represent the threshold type.
The BSP is considered eligible to volunteer if the threshold result is less than or equal to the threshold to succeed.

Formulas:

1. **Threshold Global Starting Point ($T_{gsp}$)**: Establishes a baseline threshold for all BSPs to meet based on replication targets, global weight, and maximum threshold.

2. **Threshold Weighted Starting Point ($T_{wsp}$)**: Increases the threshold starting point of a BSP based on their weight, giving higher-weight BSPs a head start.

3. **Threshold Slope ($T_{s}$)**: Defines the rate at which the threshold increases from the starting point up to the maximum threshold over a specified number of blocks.

4. **Threshold ($T$)**: Calculates the current threshold to succeed for a BSP at a given block, combining the weighted starting point and the slope over elapsed blocks from the initiated storage request.

#### Threshold Global Starting Point

The goal here is to have half of the replication target $R_{t}$ be probabilistically volunteered for by eligible BSPs after the first block from initiating the storage request., while taking into account the cumulative weight $W_{g}$ of the entire set of BSPs which reduces the starting point.

$$T_{gsp} = \frac{1}{2} \cdot \frac{R_{t}}{W_{g}} \cdot M$$

$T_{gsp}$: _Threshold global starting point_
$R_{t}$: _Replication target_ (number of BSPs required to fulfill a storage request, otherwise known as `MaxDataServers`)
$W_{g}$: _Global weight_ (cumulative weight of all BSPs)
$M$: _Maximum threshold_ (all BSPs would be eligible to volunteer, for example `u32::MAX`)

#### Threshold Weighted Starting Point

Assuming the implemented reputation system sets all BSPs to have a starting weight of 1. Any increase in reputation (i.e. increase in weight) would give an advantage over other BSPs by multiplying their own weight to the global starting point $T_{gsp}$. This will increase their own starting point and increase the probably of succeeding the threshold check.

$$T_{wsp} = w \cdot T_{gsp}$$

$w$: _BSP weight_ (current weight of the BSP)

#### Global Threshold Slope

The rate of increase of the threshold from the global starting point to the maximum threshold over a period of blocks $B_{t}$ required to reach the maximum threshold $M$.
This global threshold slope is calculated taking into account the global starting point, so that it is the same for all BSPs.

$$S_{g} = \frac{M - T_{gsp}}{B_{t}}$$

$B_{t}$: _Block time_ (number of blocks to pass to reach $M$)

#### Weighted Threshold Slope

The actual rate of increase of the threshold from the weighted starting point to the maximum threshold.
This weighted threshold slope is calculated taking into account the BSP's weight, so that it is different for each BSP, and BSPs with higher weights will have a higher threshold slope.

$$S_{w} = w \cdot S_{g}$$

$w$: _BSP weight_ (current weight of the BSP)

#### Threshold

The threshold to succeed will be different for each BSP. By calculating their own starting point $T_{wsp}$ and increasing at a rate based on the global weight of all BSPs over a period of blocks $B_{t}$ required to reach the maximum threshold $M$.

$$T = T_{wsp} + S_{w} \cdot b$$

$T_{wsp}$: _Threshold weighted starting point_ (taking into account the BSP's weight)
$T_{s}$: _Threshold slope_ (rate of increase reaching constant within target block time)
$b$: _Blocks passed_ (number of blocks passed since initiated storage request)

> Note ℹ️
>
> Actually, the threshold calculation is not done using _**blocks**_, but _**ticks**_. The reason for this is to prevent a potential spamming attack, where a malicious BSP would spam the network with tipped transactions, preventing honest BSPs from volunteering first, and thus letting blocks pass until they themselves can volunteer first.
>
> For mor information on ticks, check out the [Proofs Dealer Pallet](./../proofs-dealer/README.md).
