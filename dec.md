# Architecting Decisions
## Traits or more `#[cfg(...)]` for saving platforms

- Traits like `SavingFs`
    - Pros
        - Establish a shared interface of functions to prevent mixups when switching platforms
        - Single source of truth for function signatures
    - Cons
        - Lots of dead code, and therefore `#[cfg_attr(..., allow(dead_code))]`
        - How to set default implementor for each platform?
        - Unclear, not really what traits are for.
        - Generics/dynamic dispatch might be needed
- Same name with `#[cfg]` directives
    - Pros
        - Cleaner code in the parts not relating to the exact implementations
        - Avoids a crap ton of dead code.
        - Ironically, maybe fewer `#[cfg]` directives
        - Easier modularity
    - Cons
        - No compiler-enforced single source of truth

Final choice: simple `#[cfg]` approach, no traits.
