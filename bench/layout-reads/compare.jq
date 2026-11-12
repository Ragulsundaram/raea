# One frame's text runs, raea's ($r, the render manifest's) against Chrome's ($c, boxes.ts): each raea run is paired
# with the first unpaired Chrome run of the same words, in order. Says how many paired, the words each side has alone,
# and the largest differences in px: box (x, y, width, height) and ink (x and width).
def gap(a; b; at): [at[] | ((a[.] // 0) - (b[.] // 0)) | fabs] | max // 0;
reduce $r[] as $x ({used: [], pairs: [], alone: []};
  . as $s
  | ([$c | to_entries[] | select(.value.text == $x.text) | .key | select(. as $k | $s.used | any(.[]; . == $k) | not)]) as $k
  | if ($k | length) > 0 then
      .used += [$k[0]]
      | .pairs += [{text: $x.text, box: gap($x.box; $c[$k[0]].box; [0, 1, 2, 3]),
                    ink: (if $x.ink and $c[$k[0]].ink then gap($x.ink; $c[$k[0]].ink; [0, 2]) else 0 end)}]
    else .alone += [$x.text] end)
| . as $s
| {paired: ($s.pairs | length),
   raea_only: $s.alone,
   chrome_only: [$c | to_entries[] | select(.key as $k | $s.used | any(.[]; . == $k) | not) | .value.text],
   box: ([$s.pairs[].box] | max // 0),
   ink: ([$s.pairs[].ink] | max // 0),
   worst: (($s.pairs | max_by([.box, .ink] | max)) // null)}
