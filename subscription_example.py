# type: ignore
import asyncio
import os
from datetime import datetime

from convex import ConvexClient
from dotenv import load_dotenv

load_dotenv(".env.local")
load_dotenv()
client = ConvexClient(os.environ["CONVEX_URL"])

sub1 = client.subscribe("users:list")
sub2 = client.subscribe("users:listTemp")

watch = client.watch_all()


def watch_sub_sync(sub):
    for update in sub:
        print(update)


async def watch_sub_async(sub):
    async for update in sub:
        print(update)


async def bg_watcher(watch, sub=None):
    async for update in watch:
        if sub:
            print(update[sub.id])
        else:
            print(update)


async def interrupt_every_5_sec():
    while True:
        now = datetime.now()
        current_time = now.strftime("%H:%M:%S")
        print(f"Woke up! at {current_time}")
        await asyncio.sleep(5)


loop = asyncio.new_event_loop()

task = loop.create_task(interrupt_every_5_sec())
task2 = loop.create_task(bg_watcher(watch, sub1))
# task2 = loop.create_task(watch_sub_async(sub1))
loop.run_forever()

# NOTE: On SIGINT, make sure to run task.cancel() if you want to reuse the subscription.
