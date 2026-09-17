Before implementing a new mechanism, first establish that an adequate existing solution cannot be reused.

Required order of investigation:

1. Check the language and standard library.
2. Check dependencies already present in the repository.
3. Search for maintained libraries implementing the required behavior.
4. Search for existing upstream/reference implementations.
5. Check whether an existing CLI/tool can be invoked instead of reimplemented.
6. Check whether an existing parser, solver, verifier, schema, protocol implementation, or analysis framework already covers the problem.
7. Only then implement custom logic.

The implementation must record, briefly:

* what existing solutions were checked;
* which one is closest;
* why it cannot be used directly;
* whether it can still supply parsing, IR, algorithms, test vectors, or reference behavior;
* what exact project-specific gap remains.

Do not reimplement a general-purpose parser, solver, serializer, diff engine, protocol implementation, cryptographic primitive, concurrency primitive, database engine feature, or standards logic unless the reuse investigation demonstrates a concrete mismatch.

Prefer composition over reimplementation: existing library + thin adapter is better than project-specific machinery.

For analyzers and verification tools, unsupported input must never silently become negative evidence. Unknown or unhandled semantics must produce an explicit INCONCLUSIVE / OPAQUE / REFUSED result.

For new analysis logic, require evidence in this order where applicable:

* historical bug reproducer;
* positive and negative golden tests;
* property-based tests;
* mutation testing;
* fuzzing or corpus testing;
* bounded model checking;
* solver/formal verification only where the core property justifies the cost.

Every detector must have at least one positive control proving that the detector itself is alive.

Before making broad claims, run a real-corpus census and measure unsupported/opaque cases rather than assuming they are rare.

A frozen layer is reopened only by a concrete counterexample, invalidated assumption, or downstream requirement, not for naming cleanup or speculative completeness.
