public class Input {
    public void method() {
        // Binary operators at end of line - should move to next line
        int sum = 1
            + 2;
        int diff = 10
            - 3;
        int product = 5
            * 4;

        // Comparison operators at end of line
        boolean test = sum
            > diff;

        // Logical operators at end of line
        boolean logic = true
            && false;

        // Ternary operator at end of line
        int result = test
            ? 1 : 2;

        // Assignment compound operators (not in default tokens, unchanged)
        sum +=
            10;
    }
}
