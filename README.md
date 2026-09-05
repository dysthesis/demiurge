Demiurge - a personal static site generator
===========================================

This repository contains my work-in-progress static site generator. The
architecture for it is loosely based on the definition of a build system
described in [Build systems à la carte] -- it can be considered as a build
system with a verifying trace rebuilder, as well as a topological scheduler
for its applicative tasks (_i.e._, dependencies can be statically known, without
running the task itself).

> _"Everything should be made as simple as possible, but not simpler."_
>
> – Albert Einstein.

The intent is to make it as simple as possible, precisely covering my need,
while being amenable to future modifications should said need change. As of
writing this, said needs are as follows:

- have the source files be written in Markdown[^1],
- highlight the syntax of code blocks,
- render LaTeX blocks,
- produce a site that is as lightweight and efficient as possible, with as
  little CSS[^2] -- and even more so, Javascript -- as possible, and
- render it as quickly as possible.[^3]

[Build systems à la carte]: https://dl.acm.org/doi/10.1145/3236774
[^1]: I have explored the possibility of writing it in Typst, but consider its
current state of HTML export capabilities unsatisfactory. There is no crate
providing a stable Rust interface to the compiler; the solution would be to
shell out to the Typst CLI to generate an HTML file, and perform HTML rewriting.
This both seems inefficient and doesn't suit my tastes, and for now, Markdown
satisfies my need just fine anyways. I may revisit this if it's ever necessary,
and Demiurge's architecture makes it relatively painless to extend it this way,
anywys.
[^2]: The aim is to have the least CSS possible while still maximising general
readability; though, I may slightly splurge on
[Tufte CSS](https://edwardtufte.github.io/tufte-css/) (surely there exists
some definition of "readability" that strictly justifies this!).
[^3]: Not a strict requirement, obviously, I just like it to be fast.
[Tufte CSS]:
