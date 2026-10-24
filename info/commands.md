# Trufel modules commands list

## Trubeta (GPU)

- `setbackbuf(mode:n)` - enables or disables backbuffer.

if `mode > 0`, allocates backbuffer in VRAM and uses it for drawing operations.
else, destroys it and uses *screenbuffer* for drawing operations.

- `startframe()` - signals GPU about starting frame.

on calling `startframe`, GPU saves the time when it was called.
it's used in `endframe`, for calculating the sleep time. see `endframe`
for more details.

- `endframe(fps:n)` - swaps buffers and sleeps code execution.

swaps screenbuffer and backbuffer (if enabled), updating the screen contents
with the contents of backbuffer. after that, it hangs for `(1000 / fps) - (t2 - t1)`
milliseconds, where `t1` is the frame start time that must be saved via
the `startframe` command.

- `dot(x:n, y:n, c:n)` - draws dot at [`x`, `y`] with color `c`.

- `line(x1:n, y1:n, x2:n, y2:n, c:n)` - draws line, with first point at [`x1`, `y1`] and
second at [`x2`, `y2`], with color `c`.

- `box(x:n, y:n, w:n, h:n, c:n)` - drawa rectangle at [`x`, `y`] with width `w`, height `h`
and color `c`.

- `blit(src:a, sx:n, sy:n, sw:n, sh:n, dest:a, dx:n, dy:n, flags:n)` - copies `src` texture's
region into texture `dest`.

`blit` stands for *block image transfer*, so the `blit` command copies a block of
one texture and puts it into other.

`src` region is defined with `sx`, `sy`, `sw` and `sh` arguments, where `sw` is width
and `sh` is height. 
