import y_py as Y
import asyncio
from tkinter import Tk, Text, BOTH, INSERT, END
from websockets import connect
from ypy_websocket import WebsocketProvider 


class TextEditor:
    doc: Y.YDoc
    websocket_client: WebsocketProvider
    root: Tk
    textbox: Text
    text: Y.YText

    def __init__(self, id:int=0):
        self.doc = Y.YDoc(id)
        self.text = self.doc.get_text("content")
        self.text.observe(self.render_text)
        # root
        self.root = Tk()
        self.textbox = Text(self.root)
        self.textbox.bind("<KeyPress>", self.on_entry)
        self.textbox.bind("<KeyPress-BackSpace>", self.on_delete)
        self.textbox.pack(expand=True, fill=BOTH)
        self.root.minsize(300, 300)
        self.root.title("Text Editor")

        

    async def render(self):
        while True:
            self.root.update()
            await asyncio.sleep(0.01)


    def render_text(self,e: Y.YTextEvent):
        i = self.cursor_index()
        self.textbox.delete("1.0", END)
        self.textbox.insert(END, str(self.text))
        self.textbox.mark_set("insert", f"1.{i}")

    def cursor_index(self) -> int:
        line, col = [ int(s) for s in self.textbox.index(INSERT).split(".") ]
        return col


    def on_entry(self, e):
        print(e)
        if e.char == "BackSpace" and len(e.char) == 1:
            return
        char = "\n" if e.keysym == "Return" else e.char
        with self.doc.begin_transaction() as txn:
            index = self.cursor_index()
            print(index)
            self.text.insert(txn,index,char)



    def on_delete(self, e):
        with self.doc.begin_transaction() as txn:
            del_index = self.cursor_index() - 1
            if del_index < 0:
                return
            self.text.delete(txn, del_index)
            
    async def connect_ws(self, websocket_url:str = "ws://localhost:1234/my-roomname"):
        websocket = await connect(websocket_url)
        self.websocket_client = WebsocketProvider(self.doc, websocket)


async def main():
    te = TextEditor()
    await te.connect_ws()
    await te.render()

if __name__ == "__main__":
    asyncio.run(main())
    

