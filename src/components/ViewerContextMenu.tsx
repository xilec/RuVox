import { Menu } from '@mantine/core';
import { useEffect, useState, type RefObject } from 'react';
import {
  copyImageAddress,
  copyImageBitmap,
  copyLinkAddress,
  copySelection,
} from '../lib/viewerCopy';

interface MenuTarget {
  x: number;
  y: number;
  linkHref: string | null;
  imageSrc: string | null;
  selection: string;
}

interface Props {
  containerRef: RefObject<HTMLDivElement | null>;
}

/**
 * Custom right-click context menu for the viewer (viewer-copy-actions spec).
 * The webview provides no native context menu; this one shows only the copy
 * actions applicable to the click target: link address, selected text,
 * image bitmap and image address.
 */
export function ViewerContextMenu({ containerRef }: Props) {
  const [target, setTarget] = useState<MenuTarget | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleContextMenu = (e: MouseEvent) => {
      const el = e.target as HTMLElement;
      const link = el.closest('a[href]');
      const image = el.closest('img[src]');
      // Only a selection inside the viewer counts — otherwise a selection
      // made in another widget would pop "Копировать" for foreign text.
      const sel = window.getSelection();
      const selection =
        sel && sel.anchorNode && container.contains(sel.anchorNode)
          ? sel.toString()
          : '';
      // No applicable action: leave the event alone and do not open a menu.
      if (!link && !image && !selection) return;
      e.preventDefault();
      setTarget({
        x: e.clientX,
        y: e.clientY,
        linkHref: link?.getAttribute('href') ?? null,
        imageSrc: image?.getAttribute('src') ?? null,
        selection,
      });
    };

    container.addEventListener('contextmenu', handleContextMenu);
    return () => container.removeEventListener('contextmenu', handleContextMenu);
  }, [containerRef]);

  // Anchored to a 1px fixed-position dummy at the pointer coordinates:
  // Mantine Menu positions the dropdown against its target element.
  return (
    <Menu
      opened={target !== null}
      onClose={() => setTarget(null)}
      withinPortal
      position="bottom-start"
      offset={0}
    >
      <Menu.Target>
        <div
          style={{
            position: 'fixed',
            left: target?.x ?? 0,
            top: target?.y ?? 0,
            width: 1,
            height: 1,
            pointerEvents: 'none',
          }}
        />
      </Menu.Target>
      <Menu.Dropdown>
        {target?.linkHref && (
          <Menu.Item onClick={() => void copyLinkAddress(target.linkHref ?? '')}>
            Скопировать адрес ссылки
          </Menu.Item>
        )}
        {target?.selection && (
          <Menu.Item onClick={() => void copySelection(target.selection)}>
            Копировать
          </Menu.Item>
        )}
        {target?.imageSrc && (
          <>
            <Menu.Item onClick={() => void copyImageBitmap(target.imageSrc ?? '')}>
              Скопировать изображение
            </Menu.Item>
            <Menu.Item onClick={() => void copyImageAddress(target.imageSrc ?? '')}>
              Скопировать адрес изображения
            </Menu.Item>
          </>
        )}
      </Menu.Dropdown>
    </Menu>
  );
}
