const Y = require('yjs')
const WebsocketProvider = require('y-websocket').WebsocketProvider

const doc = new Y.Doc()
const ymap = doc.getMap("map")
const ws = require('ws')

const wsProvider = new WebsocketProvider(
  'ws://localhost:1234', 'my-roomname',
  doc,
  { WebSocketPolyfill: ws }
)

wsProvider.on('status', event => {
  console.log(event.status)
})

ymap.observe(ymapEvent => {
  ymapEvent.changes.keys.forEach((change, key) => {
    if (key === 'inc') {
      ymap.set('inc', ymap.get('inc') + 1)
    }
  })
})
