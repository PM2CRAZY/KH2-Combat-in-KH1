# KH2 player model through KH1's model system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: You asked for this in What is still missing(../what-is-still](#^m1)

---


> [!tip] Assistant
> You asked for this in [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), so I started it as its own thread. Research only so far, no code yet.
>
> **What we have now.** The KH2 player model has been switched off since s259. When it's on, it hides KH1 Sora's meshes in KH1's model queue. Then, after KH1's world pass, it skins KH2's P_EX100 on the CPU every frame (about 12k vertices) and draws it with our own Direct3D 12 pipeline. That's a renderer running beside KH1's. KH1's fog, Ambient fades, tint, lit pass and frame replays don't touch it, and the pause menu and replayed frames need special cases. It also uses KH2's PS2 textures, not the HD ones.
>
> **How KH1 draws a model (Steam decomp).** For the full addresses, see the notes below.
> - At load, KH1 finds the model's remastered mesh (the `.cvbl`, tag `0x4126`) and builds an HD table from it (`0x1401DC7F0` → `0x1401DC100`). It also builds GPU vertex buffers once (`0x1401DEBE0`). Type 8 meshes give one matrix per vertex. Type 9 meshes take up to 6 weights.
> - Each frame, the queue (`0x1401D4070`) copies the instance's bone matrices into a model record and writes one record per visible mesh.
> - At draw time, `0x1401D83A0` fills the mesh's matrix chunks from that copy. Then `0x1401DE700` draws chunk by chunk. So KH1 skins on the GPU, with up to 48 matrices per draw chunk.
>
> **What KH2 does.** KH2 Sora has 256 bones laid out differently from KH1 Sora's 293. The mesh is bound to those 256 bones (our MDLX reader finds one bone per vertex).
>
> **The catch.** KH1's matrix chunks look up bones by number in the instance's own skeleton. KH2's mesh can't go onto KH1's 293 bones without losing its own joints: the twist joints, face, 11 hair spikes against KH1's 14, and the cloth. The native way is a second KH1 instance that carries KH2's skeleton, which KH1's bone format holds as is. It would be queued for Sora's draw as a second model, the same way the left Keyblade already is. Sora's own instance stays for everything gameplay reads: hitboxes, lock points, effects on bones, the Keyblade bones, and events.
>
> **Plan, one system per version:**
> 1. Build the KH2 model in KH1's own format at load: a KH1 model header with KH2's 256 bones, an HD table, and matrix and draw chunks of at most 48 matrices each. The vertex buffers would be built by KH1's own setup.
> 2. Textures through KH1's texture objects. That means a cloned EntryTex holding a GX texture made by KH1's creator.
> 3. A second instance with KH2's skeleton, queued from KH1's queue for Sora's draw, with KH1's Sora meshes masked off. The pose goes in first by the simplest path (question 1).
> 4. The pause menu's Sora, the Drive Form models, then the Keyblade (question 3).
> 5. Remove our renderer's model path (`fx12::draw_mesh`, the CPU skinning, the 24 MB arena).
>
> Performance should get better, not worse. The vertex buffers are built once and the GPU does the skinning. Each frame adds one queue call and the pose.
>
> **What I couldn't check this round:**
> - Whether P_EX100 has weighted vertices that our reader drops.
> - Whether KH2's PC package has HD textures for P_EX100.
> - Whether KH1's lit pass needs normals (the MDLX vertices we read have none).
> - How many matrices KH2's own packets use.
> - Mesh type 10 and up.
>
> The notes are in `docs/kh1-kh2-model.md`, under "Native research (t480 round)".
>
> Questions:
> 1. **Where should the KH2 skeleton's pose come from?**
>    - (a) Each frame, from KH1's posed skeleton carried over (the inverse retarget we already have). This follows everything KH1 plays: blends, cutscenes and KH1 moves. Joints land about 7 units off KH2's own.
>    - (b) KH1's own skeleton pass on the second instance, with KH2's motions baked for KH2's skeleton. That gives exact KH2 joints, with (a) still used where there's no KH2 bake (cutscenes, KH1 moves).
>    - (c) No second skeleton: KH2's mesh rebound onto KH1's bones. This is the cheapest, but the mesh stretches and the extra joints are lost.
> 2. **Textures:** (a) KH2 PC's HD textures, if the package has them, or (b) the PS2 ones we read now?
> 3. **The Keyblade with the KH2 model:** (a) KH1's own, on Sora's bones as now, or (b) KH2's Keyblade models converted too, on the KH2 skeleton's weapon bones?

^m1


