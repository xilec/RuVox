"""Queue/history list widget."""

import logging

from PyQt6.QtCore import QPoint, Qt, pyqtSignal
from PyQt6.QtGui import QAction
from PyQt6.QtWidgets import (
    QFrame,
    QHBoxLayout,
    QLabel,
    QListWidget,
    QListWidgetItem,
    QMenu,
    QVBoxLayout,
    QWidget,
)

from ruvox.ui.models.entry import EntryStatus, TextEntry
from ruvox.ui.services.logging_service import safe_slot

logger = logging.getLogger(__name__)


class QueueItemWidget(QWidget):
    """Custom widget for queue list item display."""

    def __init__(self, entry: TextEntry, parent=None):
        super().__init__(parent)
        self.entry = entry
        # Transparent background so item's background shows through
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self._setup_ui()
        self._update_display()

    def _setup_ui(self) -> None:
        """Setup the item layout."""
        outer = QHBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        # Playing bar — 3px accent stripe on the left
        self.playing_bar = QFrame()
        self.playing_bar.setFixedWidth(3)
        self.playing_bar.setStyleSheet("background: transparent;")
        outer.addWidget(self.playing_bar)

        content = QVBoxLayout()
        content.setContentsMargins(8, 6, 8, 6)
        content.setSpacing(2)

        # First row: status + text preview
        top_row = QHBoxLayout()
        top_row.setSpacing(8)

        self.status_label = QLabel()
        self.status_label.setFixedWidth(20)
        top_row.addWidget(self.status_label)

        self.text_label = QLabel()
        self.text_label.setWordWrap(False)
        top_row.addWidget(self.text_label, 1)

        content.addLayout(top_row)

        # Second row: duration + timestamp
        bottom_row = QHBoxLayout()
        bottom_row.setSpacing(8)
        bottom_row.setContentsMargins(28, 0, 0, 0)  # Indent under status icon

        self.info_label = QLabel()
        self.info_label.setObjectName("info_label")
        self.info_label.setStyleSheet("font-size: 11px;")
        bottom_row.addWidget(self.info_label, 1)

        content.addLayout(bottom_row)
        outer.addLayout(content, 1)

    def _update_display(self) -> None:
        """Update display from entry data."""
        # Status icon
        status_icons = {
            EntryStatus.PENDING: "\u23f3",  # hourglass
            EntryStatus.PROCESSING: "\U0001f504",  # arrows counterclockwise
            EntryStatus.READY: "\u2713",  # check mark
            EntryStatus.ERROR: "\u2717",  # X mark
        }
        self.status_label.setText(status_icons.get(self.entry.status, "?"))

        # Text preview (first 50 chars)
        text_preview = self.entry.original_text[:60]
        if len(self.entry.original_text) > 60:
            text_preview += "..."
        # Replace newlines with spaces for single-line display
        text_preview = text_preview.replace("\n", " ").replace("\r", "")
        self.text_label.setText(text_preview)

        # Info line
        info_parts = []
        if self.entry.duration_sec is not None:
            mins = int(self.entry.duration_sec // 60)
            secs = int(self.entry.duration_sec % 60)
            info_parts.append(f"{mins}:{secs:02d}")

        created = self.entry.created_at
        info_parts.append(f"{created.day} {_month_name(created.month)} {created.hour}:{created.minute:02d}")

        self.info_label.setText(" \u2022 ".join(info_parts))

        # Error tooltip
        if self.entry.status == EntryStatus.ERROR and self.entry.error_message:
            self.setToolTip(f"Error: {self.entry.error_message}")
        else:
            self.setToolTip("")

    def set_playing(self, is_playing: bool) -> None:
        """Show/hide playing indicator (accent bar on the left)."""
        if is_playing:
            from ruvox.ui.themes import get_current_theme

            color = get_current_theme().accent
            self.playing_bar.setStyleSheet(f"background: {color};")
        else:
            self.playing_bar.setStyleSheet("background: transparent;")

    def update_entry(self, entry: TextEntry) -> None:
        """Update with new entry data."""
        self.entry = entry
        self._update_display()


def _month_name(month: int) -> str:
    """Get Russian month name abbreviation."""
    months = ["", "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек"]
    return months[month] if 1 <= month <= 12 else ""


class QueueListWidget(QListWidget):
    """Widget displaying queue/history of text entries."""

    entry_selected = pyqtSignal(TextEntry)
    entry_play_requested = pyqtSignal(TextEntry)
    entry_regenerate_requested = pyqtSignal(TextEntry)
    entry_delete_requested = pyqtSignal(TextEntry)

    def __init__(self, parent: QWidget | None = None):
        super().__init__(parent)
        self._entries: dict[str, TextEntry] = {}
        self._item_widgets: dict[str, QueueItemWidget] = {}
        self._current_playing_id: str | None = None

        self._setup_ui()
        self._connect_signals()

    def _setup_ui(self) -> None:
        """Setup list appearance."""
        self.setAlternatingRowColors(True)
        self.setSelectionMode(QListWidget.SelectionMode.SingleSelection)
        self.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)

    def _connect_signals(self) -> None:
        """Connect internal signals."""
        self.itemDoubleClicked.connect(self._on_item_double_clicked)
        self.currentItemChanged.connect(self._on_selection_changed)
        self.customContextMenuRequested.connect(self._show_context_menu)

    def update_entries(self, entries: list[TextEntry]) -> None:
        """Update the list with new entries using delta update."""
        sorted_entries = sorted(entries, key=lambda e: e.created_at, reverse=True)
        new_ids = {e.id for e in sorted_entries}
        old_ids = set(self._entries.keys())

        # Remove entries no longer present
        for entry_id in old_ids - new_ids:
            self.remove_entry(entry_id)

        # Update existing, add new at correct positions
        for i, entry in enumerate(sorted_entries):
            if entry.id in old_ids:
                self.update_entry(entry)
            else:
                item = QListWidgetItem()
                widget = QueueItemWidget(entry)
                item.setSizeHint(widget.sizeHint())
                self.insertItem(i, item)
                self.setItemWidget(item, widget)
                self._entries[entry.id] = entry
                self._item_widgets[entry.id] = widget
                if entry.id == self._current_playing_id:
                    widget.set_playing(True)

    def _add_entry_item(self, entry: TextEntry) -> None:
        """Add a single entry to the list."""
        item = QListWidgetItem(self)
        widget = QueueItemWidget(entry)

        item.setSizeHint(widget.sizeHint())
        self.addItem(item)
        self.setItemWidget(item, widget)

        self._entries[entry.id] = entry
        self._item_widgets[entry.id] = widget

        # Restore playing state
        if entry.id == self._current_playing_id:
            widget.set_playing(True)

    def add_entry(self, entry: TextEntry) -> None:
        """Add a new entry at the top."""
        # Insert at top (newest first)
        item = QListWidgetItem()
        widget = QueueItemWidget(entry)

        item.setSizeHint(widget.sizeHint())
        self.insertItem(0, item)
        self.setItemWidget(item, widget)

        self._entries[entry.id] = entry
        self._item_widgets[entry.id] = widget

    def update_entry_status(self, entry_id: str, status: EntryStatus, error_message: str | None = None) -> None:
        """Update status of an entry."""
        if entry_id in self._entries:
            self._entries[entry_id].status = status
            if error_message is not None:
                self._entries[entry_id].error_message = error_message
            if entry_id in self._item_widgets:
                self._item_widgets[entry_id].update_entry(self._entries[entry_id])

    def update_entry(self, entry: TextEntry) -> None:
        """Update an entry completely."""
        if entry.id in self._entries:
            self._entries[entry.id] = entry
            if entry.id in self._item_widgets:
                self._item_widgets[entry.id].update_entry(entry)

    def set_current_playing(self, entry_id: str | None) -> None:
        """Set which entry is currently playing (accent bar indicator)."""
        if entry_id is None:
            # Just stop the play indicator
            if self._current_playing_id and self._current_playing_id in self._item_widgets:
                self._item_widgets[self._current_playing_id].set_playing(False)
            return

        # Clear previous
        if self._current_playing_id and self._current_playing_id in self._item_widgets:
            self._item_widgets[self._current_playing_id].set_playing(False)

        self._current_playing_id = entry_id

        # Set new play indicator
        if entry_id in self._item_widgets:
            self._item_widgets[entry_id].set_playing(True)

    def remove_entry(self, entry_id: str) -> None:
        """Remove an entry from the list."""
        if entry_id not in self._entries:
            return

        # Find and remove the item
        for i in range(self.count()):
            item = self.item(i)
            widget = self.itemWidget(item)
            if isinstance(widget, QueueItemWidget) and widget.entry.id == entry_id:
                self.takeItem(i)
                break

        self._entries.pop(entry_id, None)
        self._item_widgets.pop(entry_id, None)

        if self._current_playing_id == entry_id:
            self._current_playing_id = None

    def get_selected_entry(self) -> TextEntry | None:
        """Get currently selected entry."""
        item = self.currentItem()
        if item:
            widget = self.itemWidget(item)
            if isinstance(widget, QueueItemWidget):
                return widget.entry
        return None

    @safe_slot
    def _on_item_double_clicked(self, item: QListWidgetItem) -> None:
        """Handle double-click to play."""
        widget = self.itemWidget(item)
        if isinstance(widget, QueueItemWidget):
            self.entry_play_requested.emit(widget.entry)

    @safe_slot
    def _on_selection_changed(self, current: QListWidgetItem, previous: QListWidgetItem) -> None:
        """Handle selection change."""
        if current:
            widget = self.itemWidget(current)
            if isinstance(widget, QueueItemWidget):
                self.entry_selected.emit(widget.entry)

    @safe_slot
    def _show_context_menu(self, position: QPoint) -> None:
        """Show context menu for entry."""
        item = self.itemAt(position)
        if not item:
            return

        widget = self.itemWidget(item)
        if not isinstance(widget, QueueItemWidget):
            return

        entry = widget.entry
        menu = QMenu(self)

        # Play action (only if ready)
        action_play = QAction("Воспроизвести", self)
        action_play.setEnabled(entry.status == EntryStatus.READY)
        action_play.triggered.connect(lambda: self.entry_play_requested.emit(entry))
        menu.addAction(action_play)

        menu.addSeparator()

        # Regenerate
        action_regenerate = QAction("Перегенерировать", self)
        action_regenerate.triggered.connect(lambda: self.entry_regenerate_requested.emit(entry))
        menu.addAction(action_regenerate)

        # Copy text
        action_copy = QAction("Копировать текст", self)
        action_copy.triggered.connect(lambda: self._copy_text(entry.original_text))
        menu.addAction(action_copy)

        # Copy normalized (for debugging)
        if entry.normalized_text:
            action_copy_norm = QAction("Копировать нормализованный", self)
            action_copy_norm.triggered.connect(lambda: self._copy_text(entry.normalized_text))
            menu.addAction(action_copy_norm)

        menu.addSeparator()

        # Delete
        action_delete = QAction("Удалить", self)
        action_delete.triggered.connect(lambda: self.entry_delete_requested.emit(entry))
        menu.addAction(action_delete)

        menu.exec(self.mapToGlobal(position))

    @safe_slot
    def _copy_text(self, text: str) -> None:
        """Copy text to clipboard."""
        from PyQt6.QtWidgets import QApplication

        clipboard = QApplication.clipboard()
        clipboard.setText(text)
