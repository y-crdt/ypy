import pdb
from random import random
import y_py as Y
import asyncio
from tkinter import Tk, Text, BOTH, INSERT, END, SEL
from ypy_websocket import WebsocketProvider 
import uuid
from client import YDocWSClient
import random



class TextEditor:
    doc: Y.YDoc
    websocket_client: WebsocketProvider
    root: Tk
    textbox: Text
    text: Y.YText

    def __init__(self, id:int=0):
        self.doc = Y.YDoc(id)
        self.text = self.doc.get_text("content")
        
        self._id = str(random.random())
        self.cursors = self.doc.get_map("cursors")
        self.ws_client = YDocWSClient()
        self.doc.observe_after_transaction(self.ws_client.send_updates)
        with self.doc.begin_transaction() as txn:
            self.cursors.set(txn, self._id, 0)
        self.text.observe(self.update_document_state)
        self.cursors.observe(self.update_cursor_locations)
        
        # root
        self.root = Tk()
        self.textbox = Text(self.root)
        
        self.textbox.bind("<KeyPress>", self.on_entry)
        self.textbox.bind("<KeyPress-BackSpace>", self.on_delete)
        self.textbox.bind("<Button-1>", self.on_click)

        self.textbox.tag_config("cursor", underline=True, underlinefg="#f00")
        self.textbox.pack(expand=True, fill=BOTH)
        self.root.minsize(300, 300)
        self.root.title("Text Editor")

        

    async def render(self):
        while True:
            self.root.update()
            await asyncio.sleep(0.001)
            self.ws_client.apply_updates(self.doc)


    def update_document_state(self,e: Y.YTextEvent):
        index = 0
        for delta in e.delta:
            if "retain" in delta:
                index += delta["retain"]
            elif "insert" in delta:
                start = self.text_to_doc_pos(index)
                end = self.text_to_doc_pos(index + len(delta["insert"]))
                # update text content
                self.textbox.insert(start, delta["insert"])
                index += len(delta["insert"])
            if "delete" in delta:
                # update text content
                start = self.text_to_doc_pos(index - 1)
                end = self.text_to_doc_pos(index -1 + delta["delete"])
                self.textbox.delete(start,end)

    def update_cursor_locations(self, e: Y.YMapEvent):
        self.set_document_cursor()
        updates = e.keys
        updates.pop(self._id)
        
        for k,pos in self.cursors.items():
            self.textbox.tag_remove("cursor", "1.0", END)
            start = self.text_to_doc_pos(int(pos))
            end = self.text_to_doc_pos(int(pos) + 1)
            self.textbox.tag_add("cursor", start, end)





    def user_document_position(self) -> int:
        return self.text_to_doc_pos(int(self.cursors[self._id]))

    def set_document_cursor(self):
        self.textbox.mark_set(INSERT, self.user_document_position())

    def text_to_doc_pos(self, text_pos: int) -> str:
        r = 1
        c = 0
        for i, char in enumerate(str(self.text)):
            if i == text_pos:
                return f"{r}.{c}"
            elif char == '\n':
                r +=1
                c = 0
            else:
                c += 1
        
        # catchall
        return END
    
    def doc_to_text_pos(self, doc_pos:str) -> int:
        row, col = tuple(map(int, doc_pos.split(".")))
        row_count = 1
        if row == row_count:
            return col
        for i, c in enumerate(str(self.text)):
            if row_count == row:
                return i + col
            if c == "\n":
                row_count += 1
        raise Exception(f"Position does not exist: {doc_pos}")

    def move_cursor(self, offset:int):
        with self.doc.begin_transaction() as txn:
            new_pos = clamp(int(self.cursors[self._id]) + offset, 0, len(self.text))
            self.cursors.set(txn, self._id, new_pos) 


    def on_entry(self, e):
        selection_tags = self.textbox.tag_ranges(SEL)
        if len(selection_tags) == 2:
            self.delete_selected(selection_tags)
        i = int(self.cursors[self._id])
        if len(e.char) == 0:
            if e.keysym == "Left":
                self.move_cursor(-1)
            elif e.keysym == "Right":
                self.move_cursor(1)
            elif e.keysym == "Up":
                with self.doc.begin_transaction() as txn:
                    self.cursors.set(txn, self._id, 0)
            elif e.keysym == "Down":
                with self.doc.begin_transaction() as txn:
                    self.cursors.set(txn, self._id, len(self.text))

        else:
            with self.doc.begin_transaction() as txn:
                self.text.insert(txn,i,e.char)
            self.move_cursor(1)
        return "break"

    def delete_selected(self, selection_tags):
        start = self.doc_to_text_pos(str(selection_tags[0]))
        end = self.doc_to_text_pos(str(selection_tags[1]))
        with self.doc.begin_transaction() as txn:
            self.text.delete_range(txn,start, end - start)
            self.cursors.set(txn, self._id, start)


    def on_delete(self, e):
        selection_tags = self.textbox.tag_ranges(SEL)
        if len(selection_tags) == 2:
            self.delete_selected(selection_tags)
        else:   
            i = clamp(int(self.cursors[self._id]), 0, len(self.text) - 1)
            if i <= 0:
                return
            with self.doc.begin_transaction() as txn:
                self.text.delete(txn, i)
            self.move_cursor(-1)
        return "break"

    def on_click(self, e):
        index = self.doc_to_text_pos(self.textbox.index(f"@{e.x},{e.y}"))
        with self.doc.begin_transaction() as txn:
            self.cursors.set(txn, self._id, index)

def clamp(val, low, high):
    return max(min(val, high), low)


async def main():
    te = TextEditor()
    asyncio.create_task(te.ws_client.start_ws_client())
    await te.render()

if __name__ == "__main__":
    asyncio.run(main())