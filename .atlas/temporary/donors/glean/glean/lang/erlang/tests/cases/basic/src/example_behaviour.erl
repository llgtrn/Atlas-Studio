-module(example_behaviour).

-callback on_event(Event :: term()) -> ok.
