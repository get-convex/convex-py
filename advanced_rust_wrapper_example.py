# type: ignore
# TODO this is currently broken
import asyncio
from datetime import datetime

from _convex import PyConvexClient

client = PyConvexClient("https://flippant-cardinal-923.convex.cloud")
client.query("users:list")
sub = client.subscribe("users:list", {})
sub2 = client.subscribe("users:listTemp", {})
w = client.watch_all()

mutation_result = client.mutation("sample_mutation:sample", {})
print(mutation_result)

action_result = client.action("sample_action:sample", {})
print(action_result)


async def my_async_func():
    async for k in sub:
        print(k)


async def my_async_func2():
    async for k in sub2:
        print(k)


async def bg_watcher():
    async for update in w:
        for k in update:
            print(update[k])
            print("\n")


async def interrupt_every_5_sec():
    while True:
        now = datetime.now()
        current_time = now.strftime("%H:%M:%S")
        print(f"Woke up! at {current_time}")
        await asyncio.sleep(5)


loop = asyncio.new_event_loop()
asyncio.set_event_loop(loop)

loop.create_task(bg_watcher())
loop.create_task(interrupt_every_5_sec())
loop.run_forever()
