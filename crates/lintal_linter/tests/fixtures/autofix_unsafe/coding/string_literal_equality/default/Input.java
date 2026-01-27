public class Input {
    public void method(String s, String t) {
        // Simple string literal on right
        if (s == "foo") {
            System.out.println("matched");
        }

        // String literal on left
        if ("bar" == s) {
            System.out.println("matched");
        }

        // Not equals
        if (s != "baz") {
            System.out.println("not baz");
        }

        // Two literals (edge case)
        if ("hello" == "world") {
            System.out.println("never");
        }

        // In assignment
        boolean result = t == "test";

        // In ternary
        String msg = s == "yes" ? "affirmative" : "negative";
    }
}
