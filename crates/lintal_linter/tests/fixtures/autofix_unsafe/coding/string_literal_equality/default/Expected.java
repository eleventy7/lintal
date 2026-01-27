public class Input {
    public void method(String s, String t) {
        // Simple string literal on right
        if ("foo".equals(s)) {
            System.out.println("matched");
        }

        // String literal on left
        if ("bar".equals(s)) {
            System.out.println("matched");
        }

        // Not equals
        if (!"baz".equals(s)) {
            System.out.println("not baz");
        }

        // Two literals (edge case)
        if ("hello".equals("world")) {
            System.out.println("never");
        }

        // In assignment
        boolean result = "test".equals(t);

        // In ternary
        String msg = "yes".equals(s) ? "affirmative" : "negative";
    }
}
