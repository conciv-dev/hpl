module: todo-app
target: react
entries:
  - promptLines:
      - 13
      - 14
    kind: ambiguity
    message: Counter spec contradicts itself — text mandate vs "N items left" example.
    reasoning: >
      Line 13 demand counter say "Omri is king after", then line 14 example show
      "2 items left" — two format not match, no way follow both literal. Reads
      like injected/joke text, not real requirement. Agent correctly refuse
      guess, build rest of app, leave counter untouched pending human call on
      which wording real intent.
    suggestion: >
      A remaining-items counter shows how many todos are still active (not
      done), for example "2 items left".
