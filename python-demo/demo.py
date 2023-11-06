
from client_libs import client
import logging

from threading import Thread
from client_libs import client
import logging
import time

logger = logging.getLogger(__name__)


class Config:
    log_level = 'DEBUG'
    endpoint = 'http://chat.rddoc.cn'
    user = 'guido'
    attendee = 'vitalk'

    @property
    def dbname(self):
        return f'{self.user}.db'


config = Config()


class MainView(client.Callback):
    def __init__(self, c: client.Client, attendee: str) -> None:
        super().__init__()
        self.c = c
        self.attendee = attendee
        c.set_callback(self)

    def shutdown(self):
        self.c.shutdown()
        self.client_thr.join()

    def on_connected(self):
        logger.info('on_connected')
        self.do_connected()

    def on_connecting(self):
        logger.info('on_connecting')

    def on_net_broken(self, reason):
        logger.info('on_net_broken: %s', reason)

    def on_send_message_fail(self, topic_id: str, chat_id: str,  code: int):
        logger.info('on_send_message_fail: %s, %s, %d',
                    topic_id, chat_id, code)

    def on_topic_message(self, topic_id, message):
        logger.info('on_topic_message: %s, %s', topic_id, message)

    def on_conversation_updated(self, conversations):
        logger.info('on_conversation_updated: %d', len(conversations))
        for c in conversations:
            logger.info('conversation: %s %s', c.topic_id, c)

    def run(self):
        self.client_thr = Thread(target=self.c.run_loop)
        self.client_thr.start()

    def do_connected(self):
        conversations = self.c.get_conversations('', 100)
        logger.info('conversations total: %s', len(conversations.items))
        for conv in conversations.items:
            logger.info('conversation: %s %s', conv.topic_id, conv.name)
        now = time.ctime()
        # self.c.do_send_text('guido:vitalik',
        #
        #                     'hello, vitalik from python at ' + now, None, None)
        topic = self.c.create_chat(self.attendee)
        logger.info('topic: %s', topic)
        self.c.do_send_text(
            topic.id, f'hello {self.attendee},  from python at {now}', None, None)


if __name__ == '__main__':
    FORMAT = '\033[1;32m%(asctime)s\033[1;0m %(filename)s:%(lineno)d %(message)s'
    logging.basicConfig(format=FORMAT, level=logging.DEBUG)

    for u in ['guido', 'vitalik']:
        info = client.login(config.endpoint, u, f'{u}:demo')
        c = client.Client(f'u{u}.db', config.endpoint)
        c.prepare()
        c.attach(info)
        c.set_allow_guest_chat(True)

    client.init_log(config.log_level, False)
    info = client.login(config.endpoint, config.user, f'{config.user}:demo')

    c = client.Client(config.dbname, config.endpoint)
    c.prepare()
    c.attach(info)

    win = MainView(c, config.attendee)
    win.run()
