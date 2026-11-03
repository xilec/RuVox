import { Menu } from '@mantine/core';
import { useEffect, useState, type RefObject } from 'react';
import {
  copyImageAddress,
  copyImageBitmap,
  copyLinkAddress,
  copySelection,
} from '../lib/viewerCopy';
import { isSingleSourceToken } from '../lib/userDictionary';
import { useT } from '../lib/i18n';

interface MenuTarget {
  x: number;
  y: number;
  linkHref: string | null;
  imageSrc: string | null;
  selection: string;
}

interface Props {
  containerRef: RefObject<HTMLDivElement | null>;
  /** Add the selected word to the pronunciation dictionary (user-dictionary
   *  spec, "Viewer quick-add"); shown only for a single valid source token. */
  onAddToDictionary?: (word: string) => void;
}

/**
 * Custom right-click context menu for the viewer (viewer-copy-actions spec).
 * The webview provides no native context menu; this one shows only the
 * actions applicable to the click target: the dictionary quick-add for a
 * single-word selection, and the copy actions (link address, selected text,
 * image bitmap and image address).
 */
export function ViewerContextMenu({ containerRef, onAddToDictionary }: Props) {
  const tt = useT();
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

  // The dictionary editor can prefill only a single valid source token
  // (user-dictionary spec, "Viewer quick-add"): a multi-word or invalid
  // selection keeps the copy-only menu.
  const dictionaryWord =
    target !== null &&
    onAddToDictionary !== undefined &&
    isSingleSourceToken(target.selection)
      ? target.selection.trim()
      : null;

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
        {dictionaryWord !== null && (
          <Menu.Item onClick={() => onAddToDictionary?.(dictionaryWord)}>
            {tt('dictionary.quickadd.button')}
          </Menu.Item>
        )}
        {target?.linkHref && (
          <Menu.Item onClick={() => void copyLinkAddress(target.linkHref ?? '')}>
            {tt('viewer.menu.copy_link_address')}
          </Menu.Item>
        )}
        {target?.selection && (
          <Menu.Item onClick={() => void copySelection(target.selection)}>
            {tt('viewer.menu.copy_selection')}
          </Menu.Item>
        )}
        {target?.imageSrc && (
          <>
            <Menu.Item onClick={() => void copyImageBitmap(target.imageSrc ?? '')}>
              {tt('viewer.menu.copy_image')}
            </Menu.Item>
            <Menu.Item onClick={() => void copyImageAddress(target.imageSrc ?? '')}>
              {tt('viewer.menu.copy_image_address')}
            </Menu.Item>
          </>
        )}
      </Menu.Dropdown>
    </Menu>
  );
}
