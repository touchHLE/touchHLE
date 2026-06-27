"""Lightweight fakes for exercising the async conversation handlers without a
real Telegram connection."""
from __future__ import annotations

import pytest


class FakeFile:
    def __init__(self, data: bytes) -> None:
        self._data = data

    async def download_as_bytearray(self) -> bytearray:
        return bytearray(self._data)


class FakeBot:
    def __init__(self) -> None:
        self.sent: list[dict] = []
        self.forwarded: list[dict] = []
        self._files: dict[str, FakeFile] = {}
        self.fail_markdown = False  # when True, parse_mode sends raise

    def register_file(self, file_id: str, data: bytes) -> None:
        self._files[file_id] = FakeFile(data)

    async def get_file(self, file_id: str) -> FakeFile:
        return self._files[file_id]

    async def send_message(self, chat_id, text, **kwargs):
        if self.fail_markdown and kwargs.get("parse_mode") is not None:
            from telegram.error import BadRequest

            raise BadRequest("can't parse entities")
        record = {"chat_id": chat_id, "text": text, **kwargs}
        self.sent.append(record)
        return record

    async def forward_message(self, **kwargs):
        self.forwarded.append(kwargs)
        return kwargs


class FakeMessage:
    def __init__(
        self,
        text=None,
        document=None,
        photo=None,
        video=None,
        animation=None,
        message_id=1,
        bot=None,
    ) -> None:
        self.text = text
        self.document = document
        self.photo = photo or []
        self.video = video
        self.animation = animation
        self.message_id = message_id
        self.replies: list[dict] = []
        self._bot = bot

    async def reply_text(self, text, **kwargs):
        if (
            self._bot is not None
            and self._bot.fail_markdown
            and kwargs.get("parse_mode") is not None
        ):
            from telegram.error import BadRequest

            raise BadRequest("can't parse entities")
        record = {"text": text, **kwargs}
        self.replies.append(record)
        return record


class FakeCallbackQuery:
    def __init__(self, data: str, message: FakeMessage) -> None:
        self.data = data
        self.message = message
        self.edits: list[str] = []

    async def answer(self):
        return None

    async def edit_message_text(self, text, **kwargs):
        self.edits.append(text)
        return text


class FakeUser:
    def __init__(self, username="tester", full_name="Test User", user_id=42) -> None:
        self.username = username
        self.full_name = full_name
        self.id = user_id


class FakeChat:
    def __init__(self, chat_id=123) -> None:
        self.id = chat_id


class FakeUpdate:
    def __init__(self, message=None, callback_query=None, user=None, chat=None) -> None:
        self.message = message
        self.callback_query = callback_query
        self.effective_user = user or FakeUser()
        self.effective_chat = chat or FakeChat()

    @property
    def effective_message(self):
        if self.message is not None:
            return self.message
        if self.callback_query is not None:
            return self.callback_query.message
        return None


class FakeContext:
    def __init__(self, bot=None, bot_data=None, user_data=None) -> None:
        self.bot = bot or FakeBot()
        self.bot_data = bot_data or {}
        self.user_data = user_data if user_data is not None else {}


class FakeDocument:
    def __init__(self, file_name=None, file_size=None, file_id="fid", mime_type=None) -> None:
        self.file_name = file_name
        self.file_size = file_size
        self.file_id = file_id
        self.mime_type = mime_type


class FakePhotoSize:
    def __init__(self, file_id="pid", file_size=1000) -> None:
        self.file_id = file_id
        self.file_size = file_size


@pytest.fixture
def bot():
    return FakeBot()


@pytest.fixture
def context(bot):
    return FakeContext(bot=bot)


def make_text_update(text, bot=None, message_id=1):
    msg = FakeMessage(text=text, message_id=message_id, bot=bot)
    return FakeUpdate(message=msg)
