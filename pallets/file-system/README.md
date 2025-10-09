# Pallet File System Pallet

## Design

### File Deletion

As a reminder, files stored by providers, be it BSPs or MSPs, can only be removed from a provider's _forest_ once they provide a proof of inclusion. The runtime is responsible for verifying the proof and removing the leaf (file) from the _forest_.

Users who wish to delete a file will initially call the `delete_files` extrinsic. This extrinsic optionally accepts a proof of inclusion of the file in a bucket's _forest_. MSPs will expose an RPC endpoint to allow users to request a proof of inclusion of a file in their _forest_.

If the proof is provided, the runtime will verify the proof and queue a priority challenge in the proofs-dealer pallet paired with `TrieRemoveMutation`, forcing all providers to provide a proof of (non-)inclusion of the challenged file. The runtime will automatically remove the file from the _forest_ for all providers who submitted a proof of inclusion.

If the proof is not provided, a pending file deletion request is created with a defined constant expiration time. The MSP who is supposed to be storing the file, will have until the expiration time to provide a proof of (non-)inclusion of the file in their _forest_. If it is a proof of non-inclusion, this means the file does not exist, and the pending file deletion request will be removed. This prevents all the providers from responding to a priority challenge for a file that does not exist. If it is a proof of inclusion, a priority challenge will be queued in the proofs-dealer pallet, following the same process described above.

Once a pending file deletion request reaches its expiration time and it has not been responded to, a priority challenge will be queued, following the same process described above.

### Volunteering: Succeeding Threshold Checks

BSPs are required to volunteer to store data for storage requests. To ensure data is probabilistically stored evenly across all BSPs, a threshold must be succeeded by each volunteering BSP.
To succeed the threshold check, the eligibility value for the storage request must be greater than the computed threshold result of the BSP.

The eligibility value of a storage request is different for every BSP since it is based on their reputation weight. Gaining more reputation will increase the eligibility value of the storage request for that BSP, increasing the probability of succeeding the threshold check.

The computed threshold result of a BSP is formalized as $H(C(P, F))$ where $H$ is the hashing function, $C$ is the concatenation function, $P$ is the provider id and $F$ is the file key.
The $n$ least significant bits of the hash result are converted into the configured runtime's threshold type, where $n$ is the number of bits required to represent the threshold type.

Formulas:

1. **Maximum Eligibility Value ($M$)**: Maximum eligibility value for a storage request, where all BSPs would be eligible to volunteer.

2. **Global Eligibility Value Starting Point ($EV_{gsp}$)**: Establishes a baseline eligibility value for all BSPs to meet based on the replication target of the storage request, the global reputation weight of all BSPs, and maximum eligibility value (which allows any BSP to volunteer successfully).

3. **Weighted Eligibility Value Starting Point ($EV_{wsp}$)**: This is the global eligibility value starting point weighted by the reputation weight of the specific BSP that's trying to volunteer. This gives BSPs with higher reputation a head start, with increased odds of succeeding the threshold check.

4. **Eligibility Value Growth Slope ($EV_{s}$)**: Rate at which the eligibility value increases from the starting point up to the maximum threshold over a specified number of ticks.

5. **Weighted Eligibility Value Growth Slope ($EV_{ws}$)**: The eligibility value growth slope weighted by the reputation weight of the BSP.

6. **Current Eligibility Value for BSP ($EV$)**: Represents the current eligibility value of a storage request for a BSP at a given tick, combining the weighted starting point and the weighted slope over the elapsed ticks since teh storage request was initiated.

#### Global Eligibility Value Starting Point

The goal of the formula for the global eligibility value starting point is to have probabilistically $R_{t}$ BSPs (where $R_{t}$ is the replication target of the storage request) be able to volunteer for the storage request from the initial tick when it was initiated. This minimizes the chances that a malicious user controlling a big part of BSPs is able to fully control the storage request, since it would have to, by chance, control exactly the $R_{t}$ BSPs that have a head start, since only one honest BSP is needed for the malicious user to not be able to hold the stored file hostage.
The global eligibility value starting point is also weighted by the global reputation weight of all BSPs $W_{g}$, which means that the initial set of eligible BSPs prioritizes higher reputation BSPs.

$$EV_{gsp} = \frac{R_{t}}{W_{g}} \cdot M$$

$EV_{gsp}$: _Global eligibility value starting point_ (baseline eligibility value for all BSPs to meet)
$R_{t}$: _Replication target_ (number of BSPs required to fulfill the storage request)
$W_{g}$: _Global reputation weight_ (cumulative reputation weight of all BSPs)
$M$: _Maximum eligibility value_ (where all BSPs would be eligible to volunteer, for example `u32::MAX`)

#### Weighted Eligibility Value Starting Point

Assuming the implemented reputation system sets all BSPs to have a starting reputation weight of 1, any increase in reputation that a BSP receives would give it an advantage over other BSPs by decreasing the global eligibility value starting point (since the cumulative reputation weight of all BSPs would increase) and then multiplying their own weight to that global starting point. This would increase their own eligibility value starting point and decrease the global eligibility value starting point, increasing its probability to succeed the threshold check.

$$EV_{wsp} = w \cdot EV_{gsp}$$

$w$: _BSP reputation weight_ (current reputation weight of the BSP)

#### Eligibility Value Growth Slope

The linear rate of increase of the eligibility value from the global starting point $EV_{gsp}$ to its maximum value $M$ over a defined period of ticks $T$.
This growth slope is calculated taking into account the global starting point, so that it is the same for all BSPs.

$$EV_{s} = \frac{M - EV_{gsp}}{T}$$

$T$: _Tick amount to reach maximum eligibility value_ (number of ticks required to reach the maximum eligibility value)

#### Weighted Eligibility Value Growth Slope

The actual rate of increase of the eligibility value from the weighted starting point to the maximum threshold.
It's the baseline eligibility value growth slope weighted by the BSP's reputation weight, so higher reputation BSPs will have their eligibility value increase faster, decreasing the time it takes for them to be eligible to volunteer.

$$EV_{ws} = w \cdot EV_{s}$$

$w$: _BSP reputation weight_ (current reputation weight of the BSP)

#### Current Eligibility Value for BSP

The current eligibility value of a storage request will be different for each BSP. By calculating their own weighted starting point $EV_{wsp}$ and increasing at a rate equal to their weighted growth slope $EV_{ws}$.

$$EV = EV_{wsp} + EV_{ws} \cdot t$$

$EV_{wsp}$: _Weighted eligibility value starting point_ (baseline eligibility value for the BSP to meet)
$EV_{ws}$: _Weighted eligibility value growth slope_ (rate at which the eligibility value increases for the BSP)
$t$: _Elapsed ticks_ (number of ticks since the storage request was initiated)

> Note ℹ️
>
> The eligibility value calculation is not done using _**blocks**_, but _**ticks**_. The reason for this is to prevent a potential spamming attack, where a malicious BSP would spam the network with tipped transactions, preventing honest BSPs from volunteering first, and thus letting blocks pass until they themselves can volunteer first.
>
> For more information on ticks, check out the [Proofs Dealer Pallet](./../proofs-dealer/README.md).
