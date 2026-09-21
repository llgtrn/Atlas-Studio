-define(MAX_RETRIES, 3).
-define(TRANSFORM(X), {ok, X}).

-record(request, {id :: integer(), payload :: binary(), retries = 0 :: integer()}).
