import os

from convex import ConvexClient, ConvexError
from dotenv import load_dotenv

load_dotenv(".env.local")
load_dotenv()
CONVEX_URL = os.getenv("CONVEX_URL")
if CONVEX_URL is None:
    raise EnvironmentError("CONVEX_URL not set")
assert CONVEX_URL

client = ConvexClient(CONVEX_URL)

print(client.query("users:list"))
print(client.action("sample_action:sample"))


try:
    result = client.mutation("sample_mutation:sampleMutationError")
    print(result)
    # use result somehow
except ConvexError as e:
    print(e)
    print(e.data)
    # use err_payload somehow
except Exception as e:
    # log on client-side, if you'd like.
    # also check out Log Streams for more information
    # (https://docs.convex.dev/production/integrations/log-streams)
    print(f"LOG: {e}")


client.set_auth("<YOUR TOKEN HERE>")
