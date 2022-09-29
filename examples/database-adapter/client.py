import y_py as Y
import asyncio
from tkinter import Tk, Text, BOTH, INSERT
from websockets import connect
from ypy_websocket import WebsocketProvider 
import asynctkinter as at
from random import randint
at.patch_unbind()


class TextEditor:
    doc: Y.YDoc
    websocket_client: WebsocketProvider
    root: Tk
    textbox: Text
    text: Y.YText

    def __init__(self, id:int=0):
        self.doc = Y.YDoc(id)
        self.text = self.doc.get_text("content")
        self.text.observe(print)

        

    def render(self):
        # root
        self.root = Tk()
        self.textbox = Text(self.root)
        # self.textbox.unbind("<KeyPress>")
        self.textbox.pack(expand=True, fill=BOTH)
        self.root.minsize(300, 300)
        self.root.title("Text Editor")
        self.register_event_handlers()
        self.root.mainloop()

    def cursor_index(self) -> int:
        line, chr = [ int(s) for s in self.textbox.index(INSERT).split(".") ]
        return chr

        

        
    
    def register_event_handlers(self):
        handlers = [self.on_click, self.on_delete, self.on_entry]
        for handler in handlers:
            at.start(handler())

    async def on_entry(self):
        while True:
            e = await at.event(self.textbox, "<KeyPress>")
            if e.char == "BackSpace" and len(e.char) == 1:
                continue
            with self.doc.begin_transaction() as txn:
                index = self.cursor_index()
                self.text.insert(txn,index,e.char)



    async def on_delete(self):
        while True:
            await at.event(self.textbox, '<KeyPress-BackSpace>')
            with self.doc.begin_transaction() as txn:
                del_index = self.cursor_index() - 1
                if del_index < 0:
                    continue
                self.text.delete(txn, del_index)
            

    
    async def on_click(self):
        while True:
            await at.event(self.textbox, '<Button>')
            # do something with cursor



    async def connect_ws(self, websocket_url:str = "ws://localhost:1234/my-roomname"):
        websocket = await connect(websocket_url)
        self.websocket_client = WebsocketProvider(self.doc, websocket)


async def main():
    te = TextEditor()
    at.start(te.connect_ws())
    te.render()

if __name__ == "__main__":
    asyncio.run(main())
    

