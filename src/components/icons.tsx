// Minimal inline SVG playback icons.  We use hand-rolled SVGs rather than a
// third-party icon set to avoid another dependency for just four glyphs and
// to keep their geometry centred inside Mantine ActionIcons (Unicode play /
// pause characters have uneven vertical metrics in most fonts, causing the
// play triangle to appear shifted below the button centre).

interface IconProps {
  size?: number;
  className?: string;
}

function svgProps(size: number, className?: string) {
  return {
    width: size,
    height: size,
    viewBox: '0 0 24 24',
    fill: 'currentColor',
    'aria-hidden': true,
    focusable: false,
    className,
  } as const;
}

export function IconPlay({ size = 16, className }: IconProps) {
  return (
    <svg {...svgProps(size, className)}>
      <path d="M8 5v14l11-7z" />
    </svg>
  );
}

export function IconPause({ size = 16, className }: IconProps) {
  return (
    <svg {...svgProps(size, className)}>
      <path d="M6 5h4v14H6zM14 5h4v14h-4z" />
    </svg>
  );
}

export function IconSkipPrev({ size = 16, className }: IconProps) {
  return (
    <svg {...svgProps(size, className)}>
      <path d="M6 6h2v12H6zM20 6v12L9 12z" />
    </svg>
  );
}

export function IconSkipNext({ size = 16, className }: IconProps) {
  return (
    <svg {...svgProps(size, className)}>
      <path d="M16 6h2v12h-2zM4 6v12l11-6z" />
    </svg>
  );
}
