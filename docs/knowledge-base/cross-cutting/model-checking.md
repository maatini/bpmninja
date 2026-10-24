# BPMN Model Checking and State-Space Reduction

## Purpose

Formal model checking explores possible executions of a process model to find
behavioral problems such as unsafe states, deadlocks, or improper completion.
Parallel BPMN paths can produce many equivalent execution interleavings, so
the number of states may grow rapidly.

This page summarizes relevant techniques and records how they relate to
BPMNinja. It is background material, not a claim that BPMNinja implements every
technique listed here.

## Partial Order Reduction (POR)

POR reduces redundant exploration by recognizing independent transitions whose
execution order does not affect the property being checked. Instead of exploring
every interleaving, a model checker explores representative schedules. Ample,
persistent, or stubborn sets select transitions for exploration; sleep sets can
avoid revisiting equivalent schedules. Dynamic POR discovers dependencies while
exploring executions.

For ample-set methods, correctness depends on conditions that preserve enabled
behavior, dependencies, property visibility, and cycles. Common presentations
call these C0–C3; implementations may use different formulations or provisos.
POR is a reduction technique, not a substitute for defining the model and
properties correctly.

## Other State-Space Techniques

These techniques address different sources of complexity and may be combined
with POR:

| Technique | Main idea | Typical use |
|-----------|-----------|-------------|
| Symbolic model checking (BDD/SAT) | Represent or solve sets of states using logical formulas rather than enumerating each state individually. | Large Boolean or hardware-oriented transition systems; bounded checking with SAT. |
| Symmetry reduction | Treat states related by permutations of interchangeable components as equivalent. | Repeated workers, peers, or identical process participants. |
| Abstraction and CEGAR | Replace concrete details with a smaller abstract model; refine it when a counterexample is spurious. | Large or infinite data domains and control-flow analysis. |
| Compositional verification | Verify components under explicit assumptions about their environment, then check that the assumptions hold. | Systems with useful component boundaries and contracts. |
| Petri-net unfolding | Represent concurrency as partial order rather than enumerating every interleaving. | Concurrent systems naturally expressed as Petri nets. |
| Sweep-line exploration | Retain only states still needed for the remaining exploration. | Reducing memory use for suitable state-space traversals. |
| Statistical model checking | Sample executions and estimate whether properties hold with a stated statistical confidence. | Large systems where exhaustive exploration is impractical and probabilistic evidence is acceptable. |

These methods have different guarantees. In particular, statistical model
checking estimates behavior from samples; it is not an exhaustive proof.
Preprocessing or hierarchical decomposition also requires preserving the
properties of interest across the transformation.

## BPMN Verification Pipeline

A typical workflow translates BPMN into a formal execution model, applies
state-space techniques, and checks properties on the resulting model. Possible
intermediate formalisms include Petri nets, PROMELA/SPIN, and rewriting-logic
models. Soundness, safeness, completion, and deadlock freedom are examples of
properties; their exact definitions depend on the formal semantics used.

Parallel gateways and asynchronous message interactions can increase the
number of execution states. Research approaches include model decomposition,
Petri-net unfolding, memory-reduction methods, and statistical analysis of
collaborations. These are alternatives or complements, not interchangeable
implementations of POR.

## BPMNinja Implementation

BPMNinja already performs deploy-time checking in `engine-server` through
`rust_bpmn_analyzer::run_with_por`, using `AmpleSetConfig::default()`. The
current checks are safeness, option to complete, proper completion, and absence
of dead activities. `MODEL_CHECK` selects `off`, `warn`, or `enforce`; the
analysis is serialized and bounded by `MODEL_CHECK_TIMEOUT_MS`. Unsupported
input can be reported as skipped, and a timeout is inconclusive rather than a
proof that the model is invalid.

See [model_check.rs](../../../engine-server/src/server/model_check.rs) for the
implementation and [shared-patterns.md](shared-patterns.md) for configuration
defaults. The research techniques above are not documented as BPMNinja
features unless the implementation is separately verified.

## References

The URLs below were supplied with the source notes for this page. Their
bibliographic metadata and individual claims have not been independently
validated as part of this documentation update.

### Partial order reduction

- [Partial order reduction overview (CMU lecture notes)](https://www.cs.cmu.edu/~emc/15817-f08/lectures/partialorder.pdf)
- [SPIN partial-order reduction paper](https://spinroot.com/spin/symposia/ws06/040.pdf)
- [Dynamic partial order reduction paper](https://arxiv.org/abs/2211.11942)
- [Combining partial-order reduction and symbolic model checking](https://lvl.info.ucl.ac.be/uploads/Publications/CombiningPartialOrderReductionAndSymbolicModelCheckingToVerifyLTLProperties/JVM_CP_NFM_2011_REGULAR.pdf)

### BPMN verification and reduction

- [BPMN verification overview (ScienceDirect)](https://www.sciencedirect.com/science/article/abs/pii/S0164121221001047)
- [Business-process verification and state-space reduction (ScienceDirect)](https://www.sciencedirect.com/science/article/abs/pii/S0306437914001239)
- [Model-checking BPMN processes (arXiv)](https://arxiv.org/html/2407.03965v1)
- [Direct verification of BPMN processes through optimized unfolding (ResearchGate)](https://www.researchgate.net/publication/261127148_Direct_Verification_of_BPMN_Processes_through_an_Optimized_Unfolding_Technique)
